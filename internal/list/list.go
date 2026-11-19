package list

import (
	"fmt"
	"maps"
	"os"
	"os/exec"
	"runtime"
	"slices"
	"strings"

	"github.com/kadinvanvalin/git-sync/internal/config"
	"github.com/kadinvanvalin/git-sync/internal/git"
	"github.com/kadinvanvalin/git-sync/internal/picker"
)

const (
	actionRemote = "Remote"
	actionClone  = "Clone"
	actionWatch  = "Watch"
)

func ViewProjects(g *git.Git, cfg config.Config) error {
	for {
		inventories, err := cfg.Inventory()
		if err != nil {
			return err
		}

		selected, ok, err := picker.Pick("Select an option > ", "50%", projectLines(inventories), false)
		if err != nil {
			return err
		}
		if !ok {
			fmt.Println("received escape code. exiting")
			return nil
		}
		repo, err := parseLine(selected[0])
		if err != nil {
			return err
		}

		actions, ok, err := picker.Pick("Action > ", "40%", []string{actionRemote, actionClone, actionWatch}, false)
		if err != nil {
			return err
		}
		if !ok {
			continue
		}

		switch actions[0] {
		case actionRemote:
			url := git.WebURL(repo)
			if err := openURL(url); err != nil {
				return err
			}
			fmt.Printf("Opened: %s\n", url)
		case actionClone:
			fmt.Println("trying to CD!!")
			if err := g.CloneRepo(repo); err != nil {
				return err
			}
			if err := cfg.AddToInventory(repo); err != nil {
				return err
			}
		case actionWatch:
			target := config.WatchTarget{Host: repo.Host, Kind: config.WatchProject, Path: repo.Slug + "/" + repo.RepoName}
			added, err := cfg.Watch(target)
			if err != nil {
				return err
			}
			if added {
				fmt.Printf("watching %s\n", target)
			} else {
				fmt.Printf("already watching %s\n", target)
			}
		}
	}
}

func projectLines(inventories map[string]config.InventoryGroups) []string {
	var lines []string
	for _, host := range slices.Sorted(maps.Keys(inventories)) {
		groups := inventories[host]
		for _, slug := range slices.Sorted(maps.Keys(groups)) {
			for _, project := range groups[slug] {
				lines = append(lines, fmt.Sprintf("%s %s/%s", host, slug, project))
			}
		}
	}
	return lines
}

func parseLine(line string) (git.GitRepo, error) {
	host, repoPart, found := strings.Cut(line, " ")
	if !found {
		return git.GitRepo{}, fmt.Errorf("invalid selection: %s", line)
	}
	return git.ParseURL(fmt.Sprintf("git@%s:%s.git", host, repoPart))
}

func openURL(url string) error {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", url)
	case "windows":
		cmd = exec.Command("cmd", "/C", "start", "", url)
	default:
		cmd = exec.Command("xdg-open", url)
	}
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Start(); err != nil {
		return fmt.Errorf("%s failed: %w", cmd.Path, err)
	}
	// The opener's exit status does not matter, only that it ran
	_ = cmd.Wait()
	return nil
}
