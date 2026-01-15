param (
    [string]$Title
)

# Get current branch
$branch = git rev-parse --abbrev-ref HEAD

# Extract branch number
if ($branch -match "SAM-(\d+)") {
    $branchNumber = $matches[1]
} else {
    Write-Host "Branch name doesn't match 'SAM-XX'. Exiting."
    exit 1
}

# Prompt for PR title if not passed
if (-not $Title) {
    $Title = Read-Host "Enter PR title"
}

$prTitle = "SAM-$branchNumber $Title"

# Automatically commit any uncommitted changes
$gitStatus = git status --porcelain
if ($gitStatus) {
    Write-Host "Uncommitted changes detected. Committing them with message '$branchNumber'..."
    git add .
    git commit -m "$branchNumber"
}

# Push the branch to origin
Write-Host "Pushing branch $branch to origin..."
git push -u origin $branch

# Create the PR (without JSON flags)
Write-Host "Creating PR '$prTitle'..."
gh pr create --base main --head $branch --title "$prTitle" --body "$prTitle"

# Merge the PR and delete the branch
Write-Host "Merging PR..."
gh pr merge $branch --merge --delete-branch

# Switch to main and pull latest changes
git checkout main
git pull

Write-Host "✅ PR '$prTitle' created and merged!"
