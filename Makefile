.PHONY: webhook-server send help \
	notify-follow notify-sub notify-giftsub notify-donate notify-cheer notify-raid \
	notify-all notify-storm \
	build release overlays overlays-deps check run clean

APP_NAME  := streams-toolkit
EXPIRE    := 8000

# Fake usernames pool (picked via shuf)
USERNAMES := danielhe4rt cfrfreak nightbot pretzelrocks johndoe xqcL shroud pokimane
RAND_USER  = $(shell echo '$(USERNAMES)' | tr ' ' '\n' | shuf -n1)
RAND_BITS  = $(shell shuf -i 100-25000 -n1)
RAND_MONEY = $(shell shuf -i 5-500 -n1)
RAND_TIER  = $(shell echo 'Tier 1\nTier 2\nTier 3\nPrime' | shuf -n1)
RAND_MONTHS = $(shell shuf -i 1-48 -n1)
RAND_VIEWERS = $(shell shuf -i 50-15000 -n1)
RAND_GIFTS  = $(shell shuf -i 1-100 -n1)

# ──────────────────────────────────────────────
# Help
# ──────────────────────────────────────────────
help:
	@echo ""
	@echo "  ╔══════════════════════════════════════════════╗"
	@echo "  ║        streams-toolkit  Makefile             ║"
	@echo "  ╚══════════════════════════════════════════════╝"
	@echo ""
	@echo "  Build"
	@echo "    make build             Build overlays + toolkit (debug)"
	@echo "    make release           Build overlays + toolkit (release)"
	@echo "    make overlays          Build the React overlay bundle only"
	@echo "    make check             Test + clippy + overlays typecheck"
	@echo "    make run               Build overlays, then run the toolkit"
	@echo "    make clean             Remove cargo + overlays build output"
	@echo ""
	@echo "  Server"
	@echo "    make webhook-server    Run the webhook server"
	@echo "    make send              Send test webhook payload"
	@echo ""
	@echo "  Notifications (notify-send)"
	@echo "    make notify-follow     ♥  New follower alert"
	@echo "    make notify-sub        ★  Subscription alert"
	@echo "    make notify-giftsub    🎁 Gift sub alert"
	@echo "    make notify-donate     $  Donation alert"
	@echo "    make notify-cheer      ◆  Cheer / bits alert"
	@echo "    make notify-raid       ⚡ Raid alert"
	@echo "    make notify-all        Fire all alerts once"
	@echo "    make notify-storm      Rapid-fire random alerts"
	@echo ""
	@echo "  Override any value:  make notify-donate USER=shroud AMOUNT=1337"
	@echo ""

# ──────────────────────────────────────────────
# Build targets
#
# The Rust binary embeds overlays/dist at compile time (include_dir!, no
# build.rs), so the overlay bundle MUST be built before cargo. Every target that
# compiles the binary depends on `overlays` to guarantee that order.
# ──────────────────────────────────────────────

# Install overlay JS deps (idempotent; skipped if node_modules already present).
overlays-deps:
	@test -d overlays/node_modules || (echo "📦 Installing overlay deps..." && cd overlays && npm install)

# Build the embedded React overlay bundle into overlays/dist.
overlays: overlays-deps
	@echo "🎨 Building overlays..."
	@cd overlays && npm run build

# Full debug build: overlays first, then the toolkit binary.
build: overlays
	@echo "🦀 Building toolkit (debug)..."
	@cargo build
	@echo "✅ Built: target/debug/$(APP_NAME)"

# Full optimized build.
release: overlays
	@echo "🦀 Building toolkit (release)..."
	@cargo build --release
	@echo "✅ Built: target/release/$(APP_NAME)"

# Verify everything: Rust tests, clippy (warnings = errors), overlays typecheck.
check: overlays-deps
	@echo "🧪 cargo test..."
	@cargo test
	@echo "📎 cargo clippy..."
	@cargo clippy --all-targets
	@echo "🔎 overlays typecheck..."
	@cd overlays && npx tsc --noEmit
	@echo "✅ All checks passed"

# Build overlays then run the toolkit (TUI).
run: overlays
	@cargo run

# Remove build output (cargo target + overlays dist).
clean:
	@echo "🧹 Cleaning..."
	@cargo clean
	@rm -rf overlays/dist
	@echo "✅ Clean"

# ──────────────────────────────────────────────
# Server targets
# ──────────────────────────────────────────────
webhook-server:
	cargo run --bin webhook-server

send:
	@curl -X POST http://localhost:8000/webhooks \
		-H "Content-Type: application/json" \
		-d '{"clientId":"06c71c0a-5c89-4099-96bc-5dae7c01f95b","event":"new","resource":{"id":"697e638d8dda95380506d656","reference":"697e6378d7c3e53f280983be","type":"message"},"userId":"63934cd8e4def3a7cf0a870f"}'
	@echo "\n✅ Webhook payload sent!"

# ──────────────────────────────────────────────
# Stream alert notifications
# ──────────────────────────────────────────────

# ♥ Follow
notify-follow: USER ?= $(RAND_USER)
notify-follow:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=normal \
		--expire-time=$(EXPIRE) \
		--category=stream.follow \
		"♥  New Follower!" \
		"<b>$(USER)</b> just followed the channel!\nWelcome to the community."
	@echo "♥  Follow alert sent  →  $(USER)"

# ★ Subscription
notify-sub: USER ?= $(RAND_USER)
notify-sub: TIER ?= $(RAND_TIER)
notify-sub: MONTHS ?= $(RAND_MONTHS)
notify-sub:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=normal \
		--expire-time=$(EXPIRE) \
		--category=stream.sub \
		"★  New Subscriber!" \
		"<b>$(USER)</b> subscribed with <b>$(TIER)</b>!\n$(MONTHS) month(s) in a row."
	@echo "★  Sub alert sent     →  $(USER) ($(TIER), $(MONTHS)mo)"

# 🎁 Gift Sub
notify-giftsub: USER ?= $(RAND_USER)
notify-giftsub: TIER ?= $(RAND_TIER)
notify-giftsub: GIFTS ?= $(RAND_GIFTS)
notify-giftsub:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=normal \
		--expire-time=$(EXPIRE) \
		--category=stream.giftsub \
		"🎁  Gift Subs!" \
		"<b>$(USER)</b> gifted <b>$(GIFTS)x</b> $(TIER) subs!\nWhat a legend."
	@echo "🎁 GiftSub alert sent →  $(USER) ($(GIFTS)x $(TIER))"

# $ Donation
notify-donate: USER ?= $(RAND_USER)
notify-donate: AMOUNT ?= $(RAND_MONEY)
notify-donate: MSG ?= Segue o fio, bora codar!
notify-donate:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=critical \
		--expire-time=12000 \
		--category=stream.donate \
		"💲  Donation — R\$$$(AMOUNT),00" \
		"<b>$(USER)</b> donated <b>R\$$$(AMOUNT),00</b>\n\"$(MSG)\""
	@echo "💲 Donate alert sent  →  $(USER) R\$$$(AMOUNT),00 — $(MSG)"

# ◆ Cheer / Bits
notify-cheer: USER ?= $(RAND_USER)
notify-cheer: BITS ?= $(RAND_BITS)
notify-cheer: MSG ?= Take my bits!
notify-cheer:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=normal \
		--expire-time=$(EXPIRE) \
		--category=stream.cheer \
		"◆  Cheer — $(BITS) bits" \
		"<b>$(USER)</b> cheered <b>$(BITS)</b> bits!\n\"$(MSG)\""
	@echo "◆  Cheer alert sent   →  $(USER) $(BITS) bits"

# ⚡ Raid
notify-raid: USER ?= $(RAND_USER)
notify-raid: VIEWERS ?= $(RAND_VIEWERS)
notify-raid:
	@notify-send \
		--app-name="$(APP_NAME)" \
		--urgency=critical \
		--expire-time=12000 \
		--category=stream.raid \
		"⚡  Incoming Raid!" \
		"<b>$(USER)</b> is raiding with <b>$(VIEWERS)</b> viewers!\nBrace yourselves."
	@echo "⚡ Raid alert sent    →  $(USER) with $(VIEWERS) viewers"

# ──────────────────────────────────────────────
# Combo targets
# ──────────────────────────────────────────────

# Fire every alert type once
notify-all: notify-follow notify-sub notify-giftsub notify-donate notify-cheer notify-raid
	@echo ""
	@echo "✅ All stream alerts fired!"

# Rapid-fire random alerts with small delays
notify-storm:
	@echo "🌩️  Starting alert storm..."
	@for i in 1 2 3 4 5 6 7 8; do \
		target=$$(echo "notify-follow notify-sub notify-giftsub notify-donate notify-cheer notify-raid" | tr ' ' '\n' | shuf -n1); \
		$(MAKE) --no-print-directory $$target; \
		sleep 0.4; \
	done
	@echo "🌩️  Storm complete!"
