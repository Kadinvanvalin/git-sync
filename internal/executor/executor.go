package executor

import (
	"bytes"
	"errors"
	"fmt"
	"os/exec"
	"strings"
)

type CommandExecutor interface {
	Run(command string, args ...string) (string, error)
	Success(command string, args ...string) bool
}

type Real struct{}

func (Real) Run(command string, args ...string) (string, error) {
	var stdout, stderr bytes.Buffer
	cmd := exec.Command(command, args...)
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr

	if err := cmd.Run(); err != nil {
		line := strings.Join(append([]string{command}, args...), " ")
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			return "", fmt.Errorf("failed to execute command: %s: %s", line, strings.TrimSpace(stderr.String()))
		}
		return "", fmt.Errorf("failed to call command: %s: %w", line, err)
	}
	return stdout.String(), nil
}

func (Real) Success(command string, args ...string) bool {
	return exec.Command(command, args...).Run() == nil
}

type Debug struct{}

func (Debug) Run(command string, args ...string) (string, error) {
	joined := strings.Join(args, " ")
	fmt.Printf("DEBUG: Simulating execution of `%s %s`\n", command, joined)
	if command == "git" && joined == "remote get-url origin" {
		return "git@github.com:meatlore/git-remote.git", nil
	}
	return "mocked output", nil
}

func (Debug) Success(command string, args ...string) bool {
	fmt.Printf("DEBUG: Pretending `%s` with args `%s` succeeded\n", command, strings.Join(args, " "))
	return true // Always return success during debug mode
}
