---
title: "Writing a post"
date: 2024-02-03
description: "How posts and drafts work in rustoky."
tags: ["demo"]
---

New posts are scaffolded with `rustoky new-post "Title"`, which creates a
file at `content/posts/YYYY-MM-DD-<slug>.md` with a `draft: true` stub.

Drafts are skipped by a plain `rustoky build`. Pass `--drafts` to render them
locally — with a "Draft" badge on the post and the home listing — while
still keeping them out of `feed.xml` and `sitemap.xml`.
