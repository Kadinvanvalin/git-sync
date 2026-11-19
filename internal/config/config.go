package config

import (
	"bytes"
	"errors"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"slices"
	"strings"

	"github.com/BurntSushi/toml"

	"github.com/kadinvanvalin/git-sync/internal/git"
)

// InventoryGroups maps slug -> [project_name]
type InventoryGroups map[string][]string

type inventoryToml struct {
	Groups InventoryGroups `toml:"groups"`
}

type RemotesConfig struct {
	Remotes map[string]git.RemoteSettings `toml:"remotes"`
}

var requiredRemoteFields = []string{
	"token",
	"project_directory",
	"api_url",
	"watch_groups",
	"watch_projects",
	"last_pull",
	"host_kind",
}

type Config struct{}

func configDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(home, ".config", "gits"), nil
}

func configFilePath() (string, error) {
	dir, err := configDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, "config.toml"), nil
}

func inventoryPathFor(host string) (string, error) {
	dir, err := configDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(dir, host+".toml"), nil
}

func (Config) Remotes() (RemotesConfig, error) {
	var config RemotesConfig
	configPath, err := configFilePath()
	if err != nil {
		return config, err
	}

	raw, err := os.ReadFile(configPath)
	if errors.Is(err, fs.ErrNotExist) {
		return config, fmt.Errorf(`No config file found at %s.
Please create one with at least a [remotes.<name>] section.
Example:
[remotes.gitlab]
token = "GITLAB_TOKEN_ENV"
project_directory = "~/projects"
api_url = "https://gitlab.com/api/v4"
host_kind = "gitlab"
watch_groups = []
watch_projects = []
last_pull = "2025-01-01T00:00:00Z"`, configPath)
	}
	if err != nil {
		return config, fmt.Errorf("failed to read %s: %w", configPath, err)
	}

	meta, err := toml.Decode(string(raw), &config)
	if err != nil {
		return config, fmt.Errorf("failed to parse %s: %w", configPath, err)
	}
	if !meta.IsDefined("remotes") {
		return config, fmt.Errorf("failed to parse %s: missing field `remotes`", configPath)
	}
	for host := range config.Remotes {
		for _, field := range requiredRemoteFields {
			if !meta.IsDefined("remotes", host, field) {
				return config, fmt.Errorf("failed to parse %s: remotes.%s is missing field `%s`", configPath, host, field)
			}
		}
	}
	return config, nil
}

func (c Config) Inventory() (map[string]InventoryGroups, error) {
	remotes, err := c.Remotes()
	if err != nil {
		return nil, err
	}

	inventories := make(map[string]InventoryGroups, len(remotes.Remotes))
	for host := range remotes.Remotes {
		groups, err := readInventory(host)
		if err != nil {
			return nil, err
		}
		inventories[host] = groups
	}
	return inventories, nil
}

// readInventory returns an error wrapping fs.ErrNotExist when the host has
// not been synced yet.
func readInventory(host string) (InventoryGroups, error) {
	path, err := inventoryPathFor(host)
	if err != nil {
		return nil, err
	}
	raw, err := os.ReadFile(path)
	if errors.Is(err, fs.ErrNotExist) {
		return nil, fmt.Errorf("No inventory file found at %s: %w", path, err)
	}
	if err != nil {
		return nil, fmt.Errorf("failed to read %s: %w", path, err)
	}
	var inventory inventoryToml
	if _, err := toml.Decode(string(raw), &inventory); err != nil {
		return nil, fmt.Errorf("failed to parse %s: %w", path, err)
	}
	return inventory.Groups, nil
}

func (c Config) Repos() ([]git.GitRepo, error) {
	inventories, err := c.Inventory()
	if err != nil {
		return nil, err
	}

	var repos []git.GitRepo
	for host, groups := range inventories {
		for slug, projects := range groups {
			for _, project := range projects {
				repos = append(repos, git.GitRepo{Host: host, Slug: slug, RepoName: project})
			}
		}
	}
	return repos, nil
}

// WatchedRepos returns the inventory repos matching their remote's
// watch_groups (slug, including subgroups) or watch_projects (slug/name).
func (c Config) WatchedRepos() ([]git.GitRepo, error) {
	remotes, err := c.Remotes()
	if err != nil {
		return nil, err
	}
	repos, err := c.Repos()
	if err != nil {
		return nil, err
	}

	var watched []git.GitRepo
	for _, repo := range repos {
		settings := remotes.Remotes[repo.Host]
		if IsWatched(repo, settings) {
			watched = append(watched, repo)
		}
	}
	return watched, nil
}

// IsWatched reports whether the repo matches the remote's watch_groups
// (slug, including subgroups) or watch_projects (slug/name).
func IsWatched(repo git.GitRepo, settings git.RemoteSettings) bool {
	if slices.Contains(settings.WatchProjects, repo.Slug+"/"+repo.RepoName) {
		return true
	}
	for _, group := range settings.WatchGroups {
		group = strings.Trim(group, "/")
		if repo.Slug == group || strings.HasPrefix(repo.Slug, group+"/") {
			return true
		}
	}
	return false
}

func (Config) AddToInventory(repo git.GitRepo) error {
	path, err := inventoryPathFor(repo.Host)
	if err != nil {
		return err
	}

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("creating %s: %w", dir, err)
	}

	// Read existing or start fresh
	var inventory inventoryToml
	if raw, err := os.ReadFile(path); err == nil {
		if _, err := toml.Decode(string(raw), &inventory); err != nil {
			inventory = inventoryToml{}
		}
	}
	if inventory.Groups == nil {
		inventory.Groups = InventoryGroups{}
	}

	// Upsert the project under the slug, deduped and sorted for stable on-disk order
	projects := append(inventory.Groups[repo.Slug], repo.RepoName)
	slices.Sort(projects)
	inventory.Groups[repo.Slug] = slices.Compact(projects)

	var serialized bytes.Buffer
	if err := toml.NewEncoder(&serialized).Encode(inventory); err != nil {
		return fmt.Errorf("serializing inventory: %w", err)
	}

	return writeAtomic(path, serialized.Bytes())
}

func writeAtomic(path string, data []byte) error {
	tmp := path + ".tmp"
	if err := os.WriteFile(tmp, data, 0o644); err != nil {
		return fmt.Errorf("writing temp file %s: %w", tmp, err)
	}
	if err := os.Rename(tmp, path); err != nil {
		return fmt.Errorf("renaming %s -> %s: %w", tmp, path, err)
	}
	return nil
}
