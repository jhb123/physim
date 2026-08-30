setup-git:
    pre-commit install
    git config alias.fix-commit 'commit --edit --file=.git/COMMIT_EDITMSG'
    git config commit.template .gitmessage
