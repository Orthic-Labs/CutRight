---
name: instagram-pro
description: >
  Instagram workflow per brand: content calendar, post review, performance analysis, growth strategy.
  Use when user says "/instagram", "IG", "Instagram strategy", "IG content calendar", "review my IG",
  "why isn't IG working", "IG performance", "Reels strategy". In CutRight v2 live analytics are operator-provided (account connectors excluded); creation + manual-data analysis only.
---

# Instagram

## Status (CutRight v2)
- Content calendar + creation: works now (platform-native artifacts only).
- **Live analytics: operator-provided.** Account connectors and logged-in session automation are
  excluded; ask the operator for Insights screenshots/exports and analyze those.
- Bulk historical exports + publishing: excluded — no posting, scheduling, or account mutation.

## Live analytics via operator-provided evidence (CutRight v2)

Upstream drove a logged-in browser session via a host CLI to scrape the IG dashboard. That path is
excluded in CutRight v2: no account connectors, no logged-in session automation, no third-party
API tokens. Instead:

1. Ask the operator for the specific numbers or panels needed (profile dashboard, post insights,
   reels audience), as screenshots or exports.
2. Read on-screen numbers back to the operator before analyzing, so stale or wrong panels get
   flagged early.
3. If evidence is unavailable, label the analysis hypothetical.

## Always start with
1. `cutright://skill/brand {"brand_code":"<DD|RH|SS>"}` (typed result: BrandCard)
2. **Identify task:** strategy / calendar / single post / performance review / Reels-specific

## Tasks

### Content calendar (weekly/monthly)
Ask: posting frequency, content mix, themes/launches.

Default mix:
| Brand | Reels | Carousel | Single | Story |
|---|---|---|---|---|
| RH | 4/wk | 2/wk | 0 | daily 3-5 |
| DD | 3/wk | 1/wk | 1/wk | 2-3/wk |
| SS | 2/wk | 3/wk | 2/wk | 2/wk |

Output: 7- or 30-day grid with topic, format, hook, CTA, hashtag set, posting time.

### Single post creation
1. Topic + format
2. Hook test (first frame for Reel, first slide for carousel, first line for caption)
3. Caption: hook → 2-3 body lines → CTA OR question (never both)
4. Hashtags: 5-15 mid-tail in first comment
5. Generate via `cutright://skill/designer`, or the youtube reference of `cutright://skill/social` for Reel

### Performance review (with screenshots/exports)
Analyze:
- Reach vs followers (>20% healthy, <5% punished)
- Saves + shares (best signal — not likes)
- Profile visits / reach
- Follow rate / profile visits
- Comments-to-likes ratio
- Story exit rate per slide

Pattern-match last 30 posts:
- Top 3 by saves: common element?
- Bottom 3: what killed them?
- Off-brand vs on-brand: which performs better? (data > theory)

### Growth strategy
- Audit current state
- Identify ONE bottleneck (reach? CTR? bio? content-market fit?)
- 4-week experiment to test the fix

## Hashtag strategy
- 5-15 in FIRST COMMENT (clean caption)
- Mix: 30% brand/community, 50% mid-tail (10k-100k posts), 20% topic-broad
- Rotate sets weekly to avoid shadow-ban patterns

## Posting times (US-skewed)
- RH: 7-9am ET weekdays + Sun 8pm
- DD: 12-2pm ET weekdays + Sat 10am
- SS: 8-10pm ET Tue/Thu/Sun

## Why IG doesn't work for new accounts (cold truth)
- < 1k followers: algo barely shows posts to non-followers
- Reels = only path to non-follower reach
- Need 30+ posts of consistent quality before judging
- Hashtags help discovery, don't 10× small accounts
- Comments + DMs from your audience > follower count

## Output

Calendar:
```markdown
## IG Calendar — [brand] — [week of date]
| Day | Time | Format | Topic | Hook | CTA | Hashtag set |
| Mon | 8am | Reel | ... | ... | ... | Set A |
```

Review:
```markdown
## IG Review — [brand] — [period]

### Scorecard
- Reach: X% of followers (target 20%+)
- Save rate: X per 1k reach
- Profile visit → follow: X%
- Comments-to-likes: 1:X

### What worked (top 3)
- [post] — [why]

### What didn't (bottom 3)
- [post] — [why]

### One bottleneck to fix
[Specific change]

### Test plan (4 weeks)
- W1: ...
- W2: ...
```

