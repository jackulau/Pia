#!/bin/bash
set -e
TASK="" BRANCH="" BASE_BRANCH="main"
while [[ $# -gt 0 ]]; do
  case $1 in
    --task) TASK="$2"; shift 2 ;; --branch) BRANCH="$2"; shift 2 ;;
    --base) BASE_BRANCH="$2"; shift 2 ;; *) echo "Unknown: $1"; exit 1 ;;
  esac
done
[[ -z "$TASK" || -z "$BRANCH" ]] && echo "Usage: worktree-setup.sh --task <name> --branch <branch> [--base <base>]" && exit 1
GIT_ROOT=$(git worktree list | head -1 | awk '{print $1}')
WORKSPACE="$GIT_ROOT/.claude-workspace"
WORKTREE_DIR="$WORKSPACE/worktrees/$TASK/worktree"
mkdir -p "$(dirname "$WORKTREE_DIR")"
git fetch origin "$BASE_BRANCH" 2>/dev/null || git fetch origin
git worktree add -b "$BRANCH" "$WORKTREE_DIR" "origin/$BASE_BRANCH" 2>/dev/null || \
git worktree add -b "$BRANCH" "$WORKTREE_DIR" "$BASE_BRANCH"
for f in "$GIT_ROOT"/.env*; do
  [[ -f "$f" && "$(basename "$f")" != ".env.example" ]] && cp "$f" "$WORKTREE_DIR/" 2>/dev/null || true
done
cat > "$WORKSPACE/worktrees/$TASK/STATUS.yml" << EOF
id: $TASK
status: in_progress
branch: $BRANCH
worktree_path: $WORKTREE_DIR
started_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
completed_at: null
commit_sha: null
tests_passing: null
EOF
echo "Worktree ready: $WORKTREE_DIR"
