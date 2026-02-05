# Stage all changes
git add -A

# Get the last commit message (full body)
$lastMessage = git log -1 --pretty=%B

if (-not $lastMessage) {
    Write-Error "No previous commit message found."
    exit 1
}

# Commit with the previous commit message
git commit -m "$lastMessage"

# Push to the current branch
git push
