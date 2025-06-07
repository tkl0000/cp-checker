#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo "Installing CP Checker..."

# Create ~/.local/bin if it doesn't exist
mkdir -p ~/.local/bin

# Download or build the binary (this is a placeholder - adjust based on your distribution method)
if [ -f "./cp-checker" ]; then
    echo "Found local binary, installing..."
    cp ./cp-checker ~/.local/bin/cp-checker
else
    echo "No local binary found. Please build the binary first with 'go build -o cp-checker'"
    exit 1
fi

# Make the binary executable
chmod +x ~/.local/bin/cp-checker

# Add ~/.local/bin to PATH if it's not already there
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    SHELL_FILE=""
    if [ -f "$HOME/.zshrc" ]; then
        SHELL_FILE="$HOME/.zshrc"
    elif [ -f "$HOME/.bashrc" ]; then
        SHELL_FILE="$HOME/.bashrc"
    fi

    if [ ! -z "$SHELL_FILE" ]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$SHELL_FILE"
        echo -e "${GREEN}Added ~/.local/bin to PATH in $SHELL_FILE${NC}"
        echo "Please restart your shell or run: source $SHELL_FILE"
    else
        echo -e "${RED}Could not find .zshrc or .bashrc. Please add ~/.local/bin to your PATH manually.${NC}"
    fi
fi

echo -e "${GREEN}CP Checker has been installed successfully!${NC}"
echo "You can now use 'cp-checker' from anywhere in your terminal." 