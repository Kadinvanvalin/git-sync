go build -trimpath -ldflags="-s -w" -o ~/.local/bin/gits ./cmd/gits
cp ./wrapper.sh ~/.local/bin/wrapper.sh
