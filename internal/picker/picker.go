package picker

import (
	"fmt"

	fzf "github.com/junegunn/fzf/src"
)

// Pick runs fzf over items and returns the selected lines. It returns false
// when the user aborts or accepts without a match. With multi, tab selects
// several lines.
func Pick(prompt, height string, items []string, multi bool) ([]string, bool, error) {
	args := []string{"--prompt=" + prompt, "--height=" + height}
	if multi {
		args = append(args, "--multi")
	}
	options, err := fzf.ParseOptions(false, args)
	if err != nil {
		return nil, false, err
	}

	input := make(chan string, len(items))
	for _, item := range items {
		input <- item
	}
	close(input)
	output := make(chan string, len(items))
	options.Input = input
	options.Output = output

	code, err := fzf.Run(options)
	switch code {
	case fzf.ExitOk:
		close(output)
		var selected []string
		for line := range output {
			selected = append(selected, line)
		}
		return selected, true, nil
	case fzf.ExitNoMatch, fzf.ExitInterrupt:
		return nil, false, nil
	}
	if err == nil {
		err = fmt.Errorf("fzf exited with code %d", code)
	}
	return nil, false, err
}
