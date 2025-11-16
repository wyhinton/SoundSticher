#!/bin/bash

# Sound Stitch Release Script
# This script helps you create and publish releases easily

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
print_header() {
    echo -e "${BLUE}🎵 Sound Stitch Release Manager${NC}"
    echo -e "${BLUE}=================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

print_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

# Check if we're in a git repository
check_git_repo() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_error "This script must be run from a git repository"
        exit 1
    fi
}

# Check if working directory is clean
check_working_directory() {
    if [[ -n $(git status --porcelain) ]]; then
        print_warning "Working directory is not clean"
        echo "Uncommitted changes:"
        git status --short
        echo
        read -p "Do you want to continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Release cancelled"
            exit 0
        fi
    fi
}

# Get current version from package.json
get_current_version() {
    if [[ -f "package.json" ]]; then
        node -p "require('./package.json').version" 2>/dev/null || echo "0.0.0"
    else
        echo "0.0.0"
    fi
}

# Validate version format
validate_version() {
    local version=$1
    if [[ ! $version =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        print_error "Invalid version format. Use semantic versioning (e.g., 1.0.0)"
        return 1
    fi
    return 0
}

# Update version in all relevant files
update_version() {
    local version=$1
    
    print_info "Updating version to $version in all files..."
    
    # Update package.json
    if [[ -f "package.json" ]]; then
        sed -i.bak "s/\"version\": \".*\"/\"version\": \"$version\"/" package.json
        rm -f package.json.bak
        print_success "Updated package.json"
    fi
    
    # Update Cargo.toml
    if [[ -f "src-tauri/Cargo.toml" ]]; then
        sed -i.bak "s/version = \".*\"/version = \"$version\"/" src-tauri/Cargo.toml
        rm -f src-tauri/Cargo.toml.bak
        print_success "Updated src-tauri/Cargo.toml"
    fi
    
    # Update tauri.conf.json
    if [[ -f "src-tauri/tauri.conf.json" ]]; then
        sed -i.bak "s/\"version\": \".*\"/\"version\": \"$version\"/" src-tauri/tauri.conf.json
        rm -f src-tauri/tauri.conf.json.bak
        print_success "Updated src-tauri/tauri.conf.json"
    fi
}

# Create git tag and push
create_and_push_tag() {
    local version=$1
    local tag="v$version"
    
    print_info "Creating git tag $tag..."
    
    # Add updated files
    git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
    
    # Commit version changes
    git commit -m "bump version to $version" || true
    
    # Create annotated tag
    git tag -a "$tag" -m "Release $tag"
    
    print_info "Pushing changes and tag to remote..."
    git push origin main || git push origin master
    git push origin "$tag"
    
    print_success "Tag $tag created and pushed"
}

# Show release status
show_release_status() {
    local version=$1
    local tag="v$version"
    
    echo
    print_success "Release $tag has been initiated!"
    echo
    print_info "What happens next:"
    echo "  1. GitHub Actions will build your app for all platforms"
    echo "  2. Binaries will be automatically uploaded to the release"
    echo "  3. You can monitor progress at:"
    echo "     https://github.com/$(git config --get remote.origin.url | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/actions"
    echo
    print_info "The release will be available at:"
    echo "  https://github.com/$(git config --get remote.origin.url | sed 's/.*github.com[:/]\([^.]*\).*/\1/')/releases/tag/$tag"
    echo
}

# Main menu
show_menu() {
    local current_version=$(get_current_version)
    
    echo -e "${BLUE}Current version: ${YELLOW}$current_version${NC}"
    echo
    echo "Select release type:"
    echo "1) Patch release (bug fixes)"
    echo "2) Minor release (new features)"
    echo "3) Major release (breaking changes)"
    echo "4) Custom version"
    echo "5) Show current status"
    echo "6) Exit"
    echo
}

# Calculate next version
calculate_next_version() {
    local current=$1
    local type=$2
    
    IFS='.' read -r major minor patch <<< "$current"
    
    case $type in
        "patch")
            echo "$major.$minor.$((patch + 1))"
            ;;
        "minor")
            echo "$major.$((minor + 1)).0"
            ;;
        "major")
            echo "$((major + 1)).0.0"
            ;;
        *)
            echo "$current"
            ;;
    esac
}

# Main function
main() {
    print_header
    
    check_git_repo
    check_working_directory
    
    local current_version=$(get_current_version)
    
    while true; do
        show_menu
        read -p "Choose an option (1-6): " choice
        echo
        
        case $choice in
            1)
                local new_version=$(calculate_next_version "$current_version" "patch")
                print_info "Creating patch release: $current_version → $new_version"
                ;;
            2)
                local new_version=$(calculate_next_version "$current_version" "minor")
                print_info "Creating minor release: $current_version → $new_version"
                ;;
            3)
                local new_version=$(calculate_next_version "$current_version" "major")
                print_info "Creating major release: $current_version → $new_version"
                ;;
            4)
                read -p "Enter custom version (e.g., 1.2.3): " new_version
                if ! validate_version "$new_version"; then
                    continue
                fi
                print_info "Creating custom release: $current_version → $new_version"
                ;;
            5)
                print_info "Current project status:"
                echo "  Version: $current_version"
                echo "  Branch: $(git branch --show-current)"
                echo "  Last commit: $(git log -1 --pretty=format:'%h - %s (%an, %ar)')"
                echo
                continue
                ;;
            6)
                print_info "Goodbye!"
                exit 0
                ;;
            *)
                print_error "Invalid option. Please choose 1-6."
                continue
                ;;
        esac
        
        # Confirm release
        echo
        print_warning "This will:"
        echo "  • Update version in package.json, Cargo.toml, and tauri.conf.json"
        echo "  • Create a git commit with the version bump"
        echo "  • Create and push a git tag (v$new_version)"
        echo "  • Trigger GitHub Actions to build and release for all platforms"
        echo
        read -p "Continue with release $new_version? (y/N): " -n 1 -r
        echo
        
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            update_version "$new_version"
            create_and_push_tag "$new_version"
            show_release_status "$new_version"
            break
        else
            print_info "Release cancelled"
        fi
    done
}

# Run main function
main "$@"
