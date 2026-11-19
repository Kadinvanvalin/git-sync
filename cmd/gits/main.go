package main

import (
	"bufio"
	"errors"
	"fmt"
	"io/fs"
	"maps"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"

	"github.com/joho/godotenv"
	"github.com/spf13/cobra"

	"github.com/kadinvanvalin/git-sync/internal/config"
	"github.com/kadinvanvalin/git-sync/internal/executor"
	"github.com/kadinvanvalin/git-sync/internal/git"
	"github.com/kadinvanvalin/git-sync/internal/github"
	"github.com/kadinvanvalin/git-sync/internal/gitlab"
	"github.com/kadinvanvalin/git-sync/internal/list"
	"github.com/kadinvanvalin/git-sync/internal/picker"
)

const longDescription = "git-sync keeps lots of Git repos up to date with one command. " +
	"Point it at a directory (or read from a config), and it will discover repositories, " +
	"check for uncommitted changes, and run the appropriate Git operations (pull/push/fetch) across them. " +
	"It favors safety (dry-run by default, dirty-tree guards) and clarity (one compact report at the end), " +
	"so you can automate daily updates without surprises. " +
	"you can think of repos as branches - fetch gets all new repos, pull pulls everything"

type app struct {
	dryrun  bool
	yes     bool
	output  bool
	started bool
	git     *git.Git
	config  config.Config
}

// command wraps a subcommand so that errors returned once it is running are
// not reported as usage errors.
func (a *app) command(use, short string, args cobra.PositionalArgs, run func(args []string) error) *cobra.Command {
	return &cobra.Command{
		Use:   use,
		Short: short,
		Args:  args,
		RunE: func(cmd *cobra.Command, args []string) error {
			cmd.SilenceUsage = true
			a.started = true
			if a.dryrun {
				fmt.Println("running in dryrun mode")
				a.git = git.New(executor.Debug{})
			} else {
				a.git = git.New(executor.Real{})
			}
			return run(args)
		},
	}
}

func (a *app) status([]string) error {
	status, err := a.git.Status()
	if err != nil {
		return err
	}
	fmt.Println(status)
	return nil
}

func (a *app) commit(message []string) error {
	if err := a.git.Commit(strings.Join(message, " ")); err != nil {
		return err
	}
	if err := a.git.Push(); err != nil {
		return err
	}
	fmt.Println("commit")
	return nil
}

func (a *app) remote([]string) error {
	if a.output {
		_, err := a.git.RemoteURL()
		return err
	}
	return a.git.Remote()
}

func (a *app) syncWatched([]string) error {
	home, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	repos, err := a.config.WatchedRepos()
	if err != nil {
		return err
	}
	for _, repo := range repos {
		// maybe check if dir exists and delete if not a repo? idk
		info, err := os.Stat(filepath.Join(home, repo.Host, repo.Slug, repo.RepoName, ".git"))
		if err == nil && info.IsDir() {
			continue
		}
		fmt.Printf("cloning %+v\n", repo)
		if err := a.git.CloneRepo(repo); err != nil {
			return err
		}
	}
	return nil
}

func (a *app) sync([]string) error {
	_ = godotenv.Load() // Load environment variables from .env file
	remotes, err := a.config.Remotes()
	if err != nil {
		return err
	}

	var repos []git.GitRepo
	for host, settings := range remotes.Remotes {
		lastPull, err := time.Parse(time.RFC3339, settings.LastPull)
		if err != nil {
			return fmt.Errorf("failed to parse last_pull for %s: %w", host, err)
		}
		token := os.Getenv(settings.Token)

		switch settings.HostKind {
		case git.GitLab:
			projects, err := gitlab.GetAllProjects(settings.APIURL, token, lastPull)
			if err != nil {
				return err
			}
			found, err := git.ProjectsToRepos(projects)
			if err != nil {
				return err
			}
			repos = append(repos, found...)
		case git.GitHub:
			for _, username := range settings.WatchGroups {
				found, err := github.GetWatchedProjects(settings.APIURL, token, username)
				if err != nil {
					return err
				}
				repos = append(repos, found...)
			}
		}
	}

	for _, repo := range repos {
		if err := a.config.AddToInventory(repo); err != nil {
			return err
		}
	}
	return nil
}

func (a *app) list([]string) error {
	return list.ViewProjects(a.git, a.config)
}

func (a *app) watchAdd(args []string) error {
	var targets []config.WatchTarget
	if len(args) == 0 {
		candidates, err := a.config.WatchCandidates()
		if err != nil {
			return err
		}
		picked, err := pickTargets("Watch (tab for multiple) > ", candidates)
		if err != nil {
			return err
		}
		targets = picked
	}
	for _, arg := range args {
		target, matches, err := a.config.ResolveWatchTarget(arg)
		if err != nil {
			return err
		}
		if matches == 0 {
			fmt.Printf("note: nothing in the %s inventory matches %s yet, run `gits sync` to fetch it\n", target.Host, target.Path)
		}
		targets = append(targets, target)
	}

	for _, target := range targets {
		if a.dryrun {
			fmt.Printf("would watch %s\n", target)
			continue
		}
		added, err := a.config.Watch(target)
		if err != nil {
			return err
		}
		if added {
			fmt.Printf("watching %s\n", target)
		} else {
			fmt.Printf("already watching %s\n", target)
		}
	}
	if len(targets) > 0 && !a.dryrun {
		fmt.Println("run `gits sync-watched` to clone them")
	}
	return nil
}

func (a *app) watchRemove(args []string) error {
	watched, err := a.config.WatchTargets()
	if err != nil {
		return err
	}
	var targets []config.WatchTarget
	if len(args) == 0 {
		if len(watched) == 0 {
			fmt.Println("nothing is being watched")
			return nil
		}
		picked, err := pickTargets("Unwatch (tab for multiple) > ", watched)
		if err != nil {
			return err
		}
		targets = picked
	}
	for _, arg := range args {
		arg = strings.Trim(strings.Replace(arg, " ", "/", 1), "/")
		i := slices.IndexFunc(watched, func(t config.WatchTarget) bool { return t.Host+"/"+t.Path == arg })
		if i < 0 {
			return fmt.Errorf("%s is not being watched, see `gits watch list`", arg)
		}
		targets = append(targets, watched[i])
	}

	for _, target := range targets {
		if a.dryrun {
			fmt.Printf("would stop watching %s\n", target)
			continue
		}
		if _, err := a.config.Unwatch(target); err != nil {
			return err
		}
		fmt.Printf("stopped watching %s\n", target)
	}
	if len(targets) > 0 && !a.dryrun {
		fmt.Println("clones are left on disk, run `gits prune` to delete them")
	}
	return nil
}

func (a *app) watchList([]string) error {
	watched, err := a.config.WatchTargets()
	if err != nil {
		return err
	}
	if len(watched) == 0 {
		fmt.Println("nothing is being watched, add something with `gits watch add`")
	}
	for _, target := range watched {
		fmt.Println(target)
	}
	return nil
}

// pickTargets returns no targets, not an error, when the user escapes.
func pickTargets(prompt string, targets []config.WatchTarget) ([]config.WatchTarget, error) {
	byLine := make(map[string]config.WatchTarget, len(targets))
	lines := make([]string, 0, len(targets))
	for _, target := range targets {
		byLine[target.String()] = target
		lines = append(lines, target.String())
	}
	selected, ok, err := picker.Pick(prompt, "50%", lines, true)
	if err != nil || !ok {
		return nil, err
	}
	picked := make([]config.WatchTarget, 0, len(selected))
	for _, line := range selected {
		picked = append(picked, byLine[line])
	}
	return picked, nil
}

func (a *app) prune([]string) error {
	home, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	remotes, err := a.config.Remotes()
	if err != nil {
		return err
	}
	// the safety checks only read, so they run for real even in dryrun
	checker := git.New(executor.Real{})

	var doomed []string
	for _, host := range slices.Sorted(maps.Keys(remotes.Remotes)) {
		settings := remotes.Remotes[host]
		clones, err := findClones(filepath.Join(home, host))
		if err != nil {
			return err
		}
		for _, rel := range clones {
			repo := git.GitRepo{Host: host, Slug: filepath.Dir(rel), RepoName: filepath.Base(rel)}
			if config.IsWatched(repo, settings) {
				continue
			}
			dir := filepath.Join(home, host, rel)
			reason, err := checker.UnsavedWork(dir)
			if err != nil {
				fmt.Printf("keeping %s: could not check it: %v\n", dir, err)
				continue
			}
			if reason != "" {
				fmt.Printf("keeping %s: %s\n", dir, reason)
				continue
			}
			doomed = append(doomed, dir)
		}
	}

	if len(doomed) == 0 {
		fmt.Println("nothing to prune")
		return nil
	}
	fmt.Println("not watched and safe to delete:")
	for _, dir := range doomed {
		fmt.Printf("  %s\n", dir)
	}
	if a.dryrun {
		fmt.Printf("would delete %d repos\n", len(doomed))
		return nil
	}
	if !a.yes && !confirm(fmt.Sprintf("delete %d repos? [y/N] ", len(doomed))) {
		fmt.Println("nothing deleted")
		return nil
	}
	for _, dir := range doomed {
		if err := os.RemoveAll(dir); err != nil {
			return err
		}
		removeEmptyParents(filepath.Dir(dir), home)
		fmt.Printf("deleted %s\n", dir)
	}
	return nil
}

// findClones returns the paths, relative to root, of every git repo under it.
func findClones(root string) ([]string, error) {
	var clones []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if errors.Is(err, fs.ErrNotExist) && path == root {
			return fs.SkipAll
		}
		if err != nil {
			return err
		}
		if !d.IsDir() || path == root {
			return nil
		}
		if info, err := os.Stat(filepath.Join(path, ".git")); err == nil && info.IsDir() {
			rel, err := filepath.Rel(root, path)
			if err != nil {
				return err
			}
			clones = append(clones, rel)
			return filepath.SkipDir
		}
		return nil
	})
	return clones, err
}

// removeEmptyParents removes dir and its parents while they are empty,
// stopping at the host directory under home.
func removeEmptyParents(dir, home string) {
	for filepath.Dir(dir) != home && dir != home {
		if os.Remove(dir) != nil {
			return
		}
		dir = filepath.Dir(dir)
	}
}

func confirm(prompt string) bool {
	fmt.Print(prompt)
	answer, _ := bufio.NewReader(os.Stdin).ReadString('\n')
	answer = strings.ToLower(strings.TrimSpace(answer))
	return answer == "y" || answer == "yes"
}

func (a *app) rootCommand() *cobra.Command {
	root := &cobra.Command{
		Use:              "gits",
		Long:             longDescription,
		Version:          "0.1.0",
		TraverseChildren: true,
		RunE: func(*cobra.Command, []string) error {
			return errors.New("a subcommand is required")
		},
	}
	root.CompletionOptions.DisableDefaultCmd = true
	root.SetVersionTemplate("gits {{.Version}}\n")
	root.Flags().BoolVarP(&a.dryrun, "dryrun", "d", false, "print commands instead of running them")
	root.Flags().BoolVarP(&a.output, "output", "o", false, "outputs to stdout instead of opening in browser")
	root.Flags().BoolP("version", "V", false, "print version")

	commit := a.command("commit [message]...", "you probably want to pull first? yeah, we are doing that for you", cobra.ArbitraryArgs, a.commit)
	// everything after the first word is part of the commit message
	commit.Flags().SetInterspersed(false)

	watch := &cobra.Command{
		Use:   "watch",
		Short: "add or remove projects and groups from the watched set that sync-watched clones",
	}
	watch.AddCommand(
		a.command("add [host/path]...", "watch projects or groups, pick from the inventory when none are given", cobra.ArbitraryArgs, a.watchAdd),
		a.command("remove [host/path]...", "stop watching projects or groups, pick from the watched set when none are given", cobra.ArbitraryArgs, a.watchRemove),
		a.command("list", "show what is being watched", cobra.NoArgs, a.watchList),
	)

	prune := a.command("prune", "delete clones that are no longer watched, skipping any with local-only work", cobra.NoArgs, a.prune)
	prune.Flags().BoolVarP(&a.yes, "yes", "y", false, "delete without asking")

	root.AddCommand(
		watch,
		prune,
		a.command("status", "true status - git fetch and status", cobra.NoArgs, a.status),
		commit,
		a.command("remote", "opens the repo in browser", cobra.NoArgs, a.remote),
		a.command("sync", "gets all new projects from gitlab and puts in a toml for faster search", cobra.NoArgs, a.sync),
		a.command("list", "list of all projects gits knows about - so you can remote or clone them directly", cobra.NoArgs, a.list),
		a.command("sync-watched", "git pull on all watched projects", cobra.NoArgs, a.syncWatched),
	)
	return root
}

func main() {
	a := &app{}
	if err := a.rootCommand().Execute(); err != nil {
		if a.started {
			os.Exit(1)
		}
		os.Exit(2)
	}
}
