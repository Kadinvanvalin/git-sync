package gitlab

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"time"

	"github.com/kadinvanvalin/git-sync/internal/git"
)

func GetAllProjects(apiURL, privateToken string, lastPull time.Time) ([]git.Project, error) {
	var projects []git.Project
	for page := 1; ; page++ {
		fmt.Printf("Fetching page %d...", page)
		pageProjects, requestURL, err := getPage(apiURL, privateToken, page)
		if err != nil {
			return nil, err
		}
		if len(pageProjects) == 0 {
			fmt.Printf("Projects page %d is empty\n", page)
			break
		}
		createdAt, err := time.Parse(time.RFC3339, pageProjects[0].CreatedAt)
		if err != nil {
			return nil, fmt.Errorf("failed to parse json created_at: %w", err)
		}
		if createdAt.Before(lastPull) {
			fmt.Printf("have latest %s created_at: %s, last_pull: %s\n",
				requestURL, createdAt.Format(time.RFC3339), lastPull.Format(time.RFC3339))
			break
		}
		fmt.Printf("Found projects page %d\n", page)
		projects = append(projects, pageProjects...)
	}
	return projects, nil
}

func getPage(apiURL, privateToken string, page int) ([]git.Project, string, error) {
	query := url.Values{
		"per_page": {"100"},
		"page":     {strconv.Itoa(page)},
		"order_by": {"created_at"},
		"sort":     {"desc"},
	}
	requestURL := fmt.Sprintf("%s/projects?%s", apiURL, query.Encode())
	request, err := http.NewRequest(http.MethodGet, requestURL, nil)
	if err != nil {
		return nil, "", err
	}
	request.Header.Set("Private-Token", privateToken)

	response, err := http.DefaultClient.Do(request)
	if err != nil {
		return nil, "", err
	}
	defer response.Body.Close()

	if response.StatusCode >= http.StatusBadRequest {
		err := fmt.Errorf("GET %s: %s", requestURL, response.Status)
		fmt.Printf("Check vpn connection? %v\n", err)
		return nil, "", err
	}

	var projects []git.Project
	if err := json.NewDecoder(response.Body).Decode(&projects); err != nil {
		return nil, "", fmt.Errorf("decoding %s: %w", requestURL, err)
	}
	return projects, requestURL, nil
}
