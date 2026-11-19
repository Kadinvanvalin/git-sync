package git

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/kadinvanvalin/git-sync/internal/executor"
)

var (
	validSSHURL = regexp.MustCompile(`(git)@([^/:]+):([^/:]+)/(.+)(.git)`)
	sshURLParts = regexp.MustCompile(`(git)@([^/:]+):(.+)/([^/:]+)(.git)`)
)

type Project struct {
	SSHURLToRepo string `json:"ssh_url_to_repo"`
	CreatedAt    string `json:"created_at"`
}

type HostKind string

const (
	GitHub HostKind = "github"
	GitLab HostKind = "gitlab"
)

func (k *HostKind) UnmarshalText(text []byte) error {
	switch kind := HostKind(text); kind {
	case GitHub, GitLab:
		*k = kind
		return nil
	default:
		return fmt.Errorf("unknown host_kind %q, expected %q or %q", text, GitHub, GitLab)
	}
}

type RemoteSettings struct {
	Token            string   `toml:"token"`
	ProjectDirectory string   `toml:"project_directory"`
	APIURL           string   `toml:"api_url"`
	WatchGroups      []string `toml:"watch_groups"`
	WatchProjects    []string `toml:"watch_projects"`
	LastPull         string   `toml:"last_pull"`
	HostKind         HostKind `toml:"host_kind"`
}

type GitRepo struct {
	Host     string
	Slug     string
	RepoName string
}

type Git struct {
	executor executor.CommandExecutor
}

func New(executor executor.CommandExecutor) *Git {
	return &Git{executor: executor}
}

func (g *Git) CloneRepo(repo GitRepo) error {
	home, err := os.UserHomeDir()
	if err != nil {
		return err
	}
	if _, err := g.executor.Run("mkdir", "-p", filepath.Join(home, repo.Host, repo.Slug)); err != nil {
		return err
	}
	target := filepath.Join(home, repo.Host, repo.Slug, repo.RepoName)
	source := fmt.Sprintf("git@%s:%s/%s.git", repo.Host, repo.Slug, repo.RepoName)
	if _, err := g.executor.Run("git", "clone", source, target); err != nil {
		return err
	}
	// wrapper.sh looks for this line to change directory
	fmt.Printf("cd %s\n", target)
	return nil
}

func (g *Git) Push() error {
	stdout, err := g.executor.Run("git", "push")
	if err != nil {
		return err
	}
	fmt.Printf("Pushing: %s\n", stdout)
	return nil
}

func (g *Git) Remote() error {
	url, err := g.RemoteURL()
	if err != nil {
		return err
	}
	_, err = g.executor.Run("open", url)
	return err
}

func (g *Git) RemoteURL() (string, error) {
	url, err := g.executor.Run("git", "remote", "get-url", "origin")
	if err != nil {
		return "", err
	}
	url = strings.TrimSpace(url)
	if !validSSHURL.MatchString(url) {
		return url, nil
	}
	repo, err := ParseURL(url)
	if err != nil {
		return "", err
	}
	return WebURL(repo), nil
}

func (g *Git) Commit(message string) error {
	trunk, err := g.findTrunk()
	if err != nil {
		return err
	}
	if _, err := g.executor.Run("git", "fetch", "origin", trunk); err != nil {
		return err
	}
	lastSharedCommit, err := g.executor.Run("git", "merge-base", "HEAD", "origin/"+trunk)
	if err != nil {
		return err
	}
	lastCommitTrunk, err := g.executor.Run("git", "rev-parse", "origin/"+trunk)
	if err != nil {
		return err
	}
	if lastSharedCommit != lastCommitTrunk {
		return fmt.Errorf("HEAD does not contain the latest origin/%s, pull first", trunk)
	}

	fmt.Printf("git commit -m %s\n", message)
	_, err = g.executor.Run("git", "commit", "-m", message)
	return err
}

func (g *Git) Status() (string, error) {
	return g.executor.Run("git", "status")
}

func (g *Git) findTrunk() (string, error) {
	possibleTrunks := []string{"main", "master"}
	for _, trunk := range possibleTrunks {
		if g.executor.Success("git", "show-ref", "--verify", "refs/heads/"+trunk) {
			return trunk, nil
		}
	}
	return "", fmt.Errorf(
		"something happened while looking for trunk: %v. Some assumptions: remote is origin, we don't use master AND main, one is trunk",
		possibleTrunks,
	)
}

func WebURL(repo GitRepo) string {
	return fmt.Sprintf("https://%s/%s/%s", repo.Host, repo.Slug, repo.RepoName)
}

func ParseURL(url string) (GitRepo, error) {
	caps := sshURLParts.FindStringSubmatch(url)
	if caps == nil {
		return GitRepo{}, fmt.Errorf("failed to parse ssh url %q", url)
	}
	return GitRepo{Host: caps[2], Slug: caps[3], RepoName: caps[4]}, nil
}

func ProjectsToRepos(projects []Project) ([]GitRepo, error) {
	repos := make([]GitRepo, 0, len(projects))
	for _, project := range projects {
		repo, err := ParseURL(project.SSHURLToRepo)
		if err != nil {
			return nil, err
		}
		repos = append(repos, repo)
	}
	return repos, nil
}
