package config

import (
	"bytes"
	"errors"
	"fmt"
	"io/fs"
	"maps"
	"os"
	"slices"
	"strings"

	"github.com/BurntSushi/toml"

	"github.com/kadinvanvalin/git-sync/internal/git"
)

// WatchKind is the config.toml field a watch entry lives in.
type WatchKind string

const (
	WatchGroup   WatchKind = "watch_groups"
	WatchProject WatchKind = "watch_projects"
)

type WatchTarget struct {
	Host string
	Kind WatchKind
	Path string
}

func (t WatchTarget) String() string {
	kind := "group"
	if t.Kind == WatchProject {
		kind = "project"
	}
	return fmt.Sprintf("%s/%s (%s)", t.Host, t.Path, kind)
}

// ResolveWatchTarget turns "host/path" into a watch target. The path is a
// project when the host's inventory has it, otherwise a group (or, for
// GitHub, a username). matches is how many inventory repos it covers.
func (c Config) ResolveWatchTarget(arg string) (target WatchTarget, matches int, err error) {
	remotes, err := c.Remotes()
	if err != nil {
		return target, 0, err
	}
	arg = strings.Trim(strings.Replace(arg, " ", "/", 1), "/")
	for host := range remotes.Remotes {
		if path, ok := strings.CutPrefix(arg, host+"/"); ok && path != "" {
			target = WatchTarget{Host: host, Kind: WatchGroup, Path: strings.Trim(path, "/")}
			break
		}
	}
	if target.Host == "" {
		return target, 0, fmt.Errorf("%q does not start with a known remote (%s)", arg,
			strings.Join(slices.Sorted(maps.Keys(remotes.Remotes)), ", "))
	}

	groups, err := readInventory(target.Host)
	if err != nil && !errors.Is(err, fs.ErrNotExist) {
		return target, 0, err
	}
	settings := git.RemoteSettings{WatchGroups: []string{target.Path}}
	for slug, projects := range groups {
		for _, project := range projects {
			repo := git.GitRepo{Host: target.Host, Slug: slug, RepoName: project}
			if slug+"/"+project == target.Path {
				return WatchTarget{Host: target.Host, Kind: WatchProject, Path: target.Path}, 1, nil
			}
			if IsWatched(repo, settings) {
				matches++
			}
		}
	}
	return target, matches, nil
}

// WatchTargets lists every entry in every remote's watch lists.
func (c Config) WatchTargets() ([]WatchTarget, error) {
	remotes, err := c.Remotes()
	if err != nil {
		return nil, err
	}
	var targets []WatchTarget
	for _, host := range slices.Sorted(maps.Keys(remotes.Remotes)) {
		settings := remotes.Remotes[host]
		for _, group := range settings.WatchGroups {
			targets = append(targets, WatchTarget{Host: host, Kind: WatchGroup, Path: group})
		}
		for _, project := range settings.WatchProjects {
			targets = append(targets, WatchTarget{Host: host, Kind: WatchProject, Path: project})
		}
	}
	return targets, nil
}

// Watch adds the target to config.toml, returning false if it was already there.
func (c Config) Watch(target WatchTarget) (bool, error) {
	return editWatchList(target, func(list []string) ([]string, bool) {
		if slices.Contains(list, target.Path) {
			return list, false
		}
		return append(list, target.Path), true
	})
}

// Unwatch removes the target from config.toml, returning false if it was not there.
func (c Config) Unwatch(target WatchTarget) (bool, error) {
	return editWatchList(target, func(list []string) ([]string, bool) {
		kept := slices.DeleteFunc(slices.Clone(list), func(entry string) bool {
			return strings.Trim(entry, "/") == target.Path
		})
		return kept, len(kept) != len(list)
	})
}

// editWatchList rewrites config.toml through a generic map so fields gits
// does not know about survive. Comments and key order do not.
func editWatchList(target WatchTarget, edit func([]string) ([]string, bool)) (bool, error) {
	path, err := configFilePath()
	if err != nil {
		return false, err
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return false, fmt.Errorf("failed to read %s: %w", path, err)
	}
	var doc map[string]any
	if _, err := toml.Decode(string(raw), &doc); err != nil {
		return false, fmt.Errorf("failed to parse %s: %w", path, err)
	}
	remotes, _ := doc["remotes"].(map[string]any)
	remote, ok := remotes[target.Host].(map[string]any)
	if !ok {
		return false, fmt.Errorf("no [remotes.%q] in %s", target.Host, path)
	}

	var list []string
	if values, ok := remote[string(target.Kind)].([]any); ok {
		for _, value := range values {
			if entry, ok := value.(string); ok {
				list = append(list, entry)
			}
		}
	}
	list, changed := edit(list)
	if !changed {
		return false, nil
	}
	if list == nil {
		list = []string{}
	}
	remote[string(target.Kind)] = list

	var serialized bytes.Buffer
	if err := toml.NewEncoder(&serialized).Encode(doc); err != nil {
		return false, fmt.Errorf("serializing config: %w", err)
	}
	return true, writeAtomic(path, serialized.Bytes())
}

type WatchStatus int

const (
	// ordered by how useful the entry is to act on in the selector
	StatusDownloaded WatchStatus = iota
	StatusNotWatched
	StatusWatched
	StatusWatchedByGroup
)

type WatchEntry struct {
	WatchTarget
	Status WatchStatus
	// for groups, how many of the projects under it are cloned
	Cloned, Total int
}

// WatchEntries lists every synced group (including parent groups) and project
// with its watch status, plus watch list entries the inventory does not have
// yet. Hosts that have not been synced are skipped.
func (c Config) WatchEntries() ([]WatchEntry, error) {
	remotes, err := c.Remotes()
	if err != nil {
		return nil, err
	}

	var entries []WatchEntry
	for _, host := range slices.Sorted(maps.Keys(remotes.Remotes)) {
		settings := remotes.Remotes[host]
		inventory, err := readInventory(host)
		if errors.Is(err, fs.ErrNotExist) {
			continue
		}
		if err != nil {
			return nil, err
		}

		groups := map[string]*WatchEntry{}
		for _, slug := range slices.Sorted(maps.Keys(inventory)) {
			parts := strings.Split(slug, "/")
			var parents []*WatchEntry
			for i := range parts {
				path := strings.Join(parts[:i+1], "/")
				if groups[path] == nil {
					groups[path] = &WatchEntry{
						WatchTarget: WatchTarget{Host: host, Kind: WatchGroup, Path: path},
						Status:      groupStatus(path, settings),
					}
				}
				parents = append(parents, groups[path])
			}
			for _, project := range inventory[slug] {
				repo := git.GitRepo{Host: host, Slug: slug, RepoName: project}
				cloned := git.IsCloned(repo)
				for _, group := range parents {
					group.Total++
					if cloned {
						group.Cloned++
					}
				}
				entries = append(entries, WatchEntry{
					WatchTarget: WatchTarget{Host: host, Kind: WatchProject, Path: slug + "/" + project},
					Status:      projectStatus(repo, settings, cloned),
				})
			}
		}
		for _, group := range groups {
			if group.Status == StatusNotWatched && group.Cloned > 0 {
				group.Status = StatusDownloaded
			}
			entries = append(entries, *group)
		}
	}

	// keep watch list entries that are not synced yet, so they can be removed
	targets, err := c.WatchTargets()
	if err != nil {
		return nil, err
	}
	for _, target := range targets {
		target.Path = strings.Trim(target.Path, "/")
		if !slices.ContainsFunc(entries, func(e WatchEntry) bool { return e.WatchTarget == target }) {
			entries = append(entries, WatchEntry{WatchTarget: target, Status: StatusWatched})
		}
	}

	slices.SortFunc(entries, func(a, b WatchEntry) int {
		if a.Status != b.Status {
			return int(a.Status) - int(b.Status)
		}
		return strings.Compare(a.Host+"/"+a.Path, b.Host+"/"+b.Path)
	})
	return entries, nil
}

func groupStatus(path string, settings git.RemoteSettings) WatchStatus {
	if slices.ContainsFunc(settings.WatchGroups, func(g string) bool { return strings.Trim(g, "/") == path }) {
		return StatusWatched
	}
	if IsWatched(git.GitRepo{Slug: path}, settings) {
		return StatusWatchedByGroup
	}
	return StatusNotWatched
}

func projectStatus(repo git.GitRepo, settings git.RemoteSettings, cloned bool) WatchStatus {
	switch {
	case slices.Contains(settings.WatchProjects, repo.Slug+"/"+repo.RepoName):
		return StatusWatched
	case IsWatched(repo, settings):
		return StatusWatchedByGroup
	case cloned:
		return StatusDownloaded
	}
	return StatusNotWatched
}
