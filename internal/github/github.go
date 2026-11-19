package github

import (
	"encoding/json"
	"fmt"
	"net/http"
	"regexp"
	"strings"

	"github.com/kadinvanvalin/git-sync/internal/git"
)

var apiURLHost = regexp.MustCompile(`https?://(.+)`)

type repoResponse struct {
	FullName string `json:"full_name"`
}

func apiURLToHost(apiURL string) (string, error) {
	caps := apiURLHost.FindStringSubmatch(apiURL)
	if caps == nil {
		return "", fmt.Errorf("failed to find host in api url %q", apiURL)
	}
	return caps[1], nil
}

func GetWatchedProjects(apiURL, privateToken, user string) ([]git.GitRepo, error) {
	host, err := apiURLToHost(apiURL)
	if err != nil {
		return nil, err
	}

	requestURL := fmt.Sprintf("%s/users/%s/repos", apiURL, user)
	request, err := http.NewRequest(http.MethodGet, requestURL, nil)
	if err != nil {
		return nil, err
	}
	request.Header.Set("Accept", "application/vnd.github+json")
	request.Header.Set("X-GitHub-Api-Version", "2022-11-28")
	if privateToken != "" {
		request.Header.Set("Authorization", "Bearer: "+privateToken)
	}

	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()

	if response.StatusCode >= http.StatusBadRequest {
		err := fmt.Errorf("GET %s: %s", requestURL, response.Status)
		fmt.Printf("Check vpn connection? %v\n", err)
		return nil, err
	}

	var projects []repoResponse
	if err := json.NewDecoder(response.Body).Decode(&projects); err != nil {
		return nil, fmt.Errorf("decoding %s: %w", requestURL, err)
	}

	repos := make([]git.GitRepo, 0, len(projects))
	for _, project := range projects {
		parts := strings.Split(project.FullName, "/")
		repos = append(repos, git.GitRepo{Host: host, Slug: user, RepoName: parts[len(parts)-1]})
	}
	return repos, nil
}
