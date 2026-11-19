package main

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
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
)

const longDescription = "git-sync keeps lots of Git repos up to date with one command. " +
	"Point it at a directory (or read from a config), and it will discover repositories, " +
	"check for uncommitted changes, and run the appropriate Git operations (pull/push/fetch) across them. " +
	"It favors safety (dry-run by default, dirty-tree guards) and clarity (one compact report at the end), " +
	"so you can automate daily updates without surprises. " +
	"you can think of repos as branches - fetch gets all new repos, pull pulls everything"

type app struct {
	dryrun  bool
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
	repos, err := a.config.Repos()
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

	root.AddCommand(
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
