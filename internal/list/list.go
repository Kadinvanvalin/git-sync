package list

import (
	"fmt"
	"maps"
	"os"
	"os/exec"
	"runtime"
	"slices"
	"strings"

	fzf "github.com/junegunn/fzf/src"

	"github.com/kadinvanvalin/git-sync/internal/config"
	"github.com/kadinvanvalin/git-sync/internal/git"
)

const (
	actionRemote = "Remote"
	actionClone  = "Clone"
)

func ViewProjects(g *git.Git, cfg config.Config) error {
	for {
		inventories, err := cfg.Inventory()
		if err != nil {
			return err
		}

		selected, ok, err := pick("Select an option > ", "50%", projectLines(inventories))
		if err != nil {
			return err
		}
		if !ok {
			fmt.Println("received escape code. exiting")
			return nil
		}
		repo, err := parseLine(selected)
		if err != nil {
			return err
		}

		action, ok, err := pick("Action > ", "40%", []string{actionRemote, actionClone})
		if err != nil {
			return err
		}
		if !ok {
			continue
		}

		switch action {
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

// pick returns false when the user aborts or accepts without a match.
func pick(prompt, height string, items []string) (string, bool, error) {
	options, err := fzf.ParseOptions(false, []string{"--prompt=" + prompt, "--height=" + height})
	if err != nil {
		return "", false, err
	}

	input := make(chan string, len(items))
	for _, item := range items {
		input <- item
	}
	close(input)
	output := make(chan string, 1)
	options.Input = input
	options.Output = output

	code, err := fzf.Run(options)
	switch code {
	case fzf.ExitOk:
		return <-output, true, nil
	case fzf.ExitNoMatch, fzf.ExitInterrupt:
		return "", false, nil
	}
	if err == nil {
		err = fmt.Errorf("fzf exited with code %d", code)
	}
	return "", false, err
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
