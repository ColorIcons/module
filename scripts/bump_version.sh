#!/bin/bash
set -e

# 获取上一个 commit message 决定 bump 类型
LAST_MSG=$(git log -1 --pretty=%B)

if [[ "$LAST_MSG" =~ BREAKING ]]; then
  TYPE="major"
elif [[ "$LAST_MSG" =~ feat ]]; then
  TYPE="minor"
else
  TYPE="patch"
fi

# Rust Cargo.toml
CURRENT_VERSION=$(grep '^version =' Cargo.toml | head -n1 | cut -d '"' -f2)
IFS='.' read -r MAJOR MINOR PATCH <<<"$CURRENT_VERSION"

case $TYPE in
major)
  MAJOR=$((MAJOR + 1))
  MINOR=0
  PATCH=0
  ;;
minor)
  MINOR=$((MINOR + 1))
  PATCH=0
  ;;
patch)
  PATCH=$((PATCH + 1))
  ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
NEW_VERSIONCODE=$(printf "%03d%02d%01d" "$MAJOR" "$MINOR" "$PATCH")

# 更新 Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# 更新 module/module.prop
sed -i "s/^version=.*/version=$NEW_VERSION/" module/module.prop
sed -i "s/^versionCode=.*/versionCode=$NEW_VERSIONCODE/" module/module.prop

cargo update

# 添加所有改动到 git，推送由 action 做
git add Cargo.toml module/module.prop Cargo.lock

echo "Bumped version to $NEW_VERSION (versionCode=$NEW_VERSIONCODE)"
