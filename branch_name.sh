#!/usr/bin/env bash
# inspired by https://stackoverflow.com/a/78182026
BRANCH_REGEX='^(feature|fix|chore)/.*$'

branch="${PRE_COMMIT_LOCAL_BRANCH#refs/heads/}"

if [ "$branch" = "HEAD" ]; then
  exit 0
fi

if ! [[ $branch =~ $BRANCH_REGEX ]]; then
   echo "Error: Invalid branch name '$branch."
   echo "You can run below commands to make a valid branch name"
   echo "git branch -m $branch feature/$branch"
   echo "git branch -m $branch fix/$branch"
   echo "git branch -m $branch chore/$branch"
   exit 1
fi
