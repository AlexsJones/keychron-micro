# keychron-micro -- install, update and remove the script pad daemon.
#
# The repo IS the installation. The binary resolves config.toml and scripts/
# relative to where it was built, so this directory has to stay where it is;
# `make install` points the machine at it rather than copying anything out.
# Moving the repo means `make update` afterwards.
#
#   make install      udev rule + service, then start it   (asks for sudo once)
#   make update       rebuild and restart
#   make uninstall    remove the service and the udev rule
#   make status       is it running?
#   make logs         follow its output
#
# Only the udev rule needs root. Everything else is your user's systemd.

SHELL := /bin/bash
.DEFAULT_GOAL := help

REPO := $(patsubst %/,%,$(dir $(abspath $(lastword $(MAKEFILE_LIST)))))
BIN := $(REPO)/target/release/keychron-micro

UNIT_NAME := keychron-micro.service
UNIT_DIR := $(HOME)/.config/systemd/user
UNIT := $(UNIT_DIR)/$(UNIT_NAME)

UDEV_RULE := 70-keychron-micro.rules
UDEV := /etc/udev/rules.d/$(UDEV_RULE)

CONFIG := $(REPO)/config.toml
PRESETS := $(REPO)/presets
# Which preset `make install` starts you on. `make install PRESET=alexsjones`
# for a full pad instead of just the demo key.
PRESET ?= default

.PHONY: help build install update uninstall reinstall status logs probe learn \
        config udev unit presets preset-use preset-save

help:
	@echo 'keychron-micro'
	@echo
	@echo '  make install      install the udev rule and service, and start it'
	@echo '  make update       rebuild and restart the running daemon'
	@echo '  make uninstall    stop it and remove the service and udev rule'
	@echo '  make reinstall    uninstall, then install'
	@echo
	@echo '  make presets      list the bindings you can start from'
	@echo '  make preset-use NAME=alexsjones    adopt one'
	@echo '  make preset-save NAME=mine         publish your config.toml as one'
	@echo
	@echo '  make status       show whether the daemon is running'
	@echo '  make logs         follow the daemon log'
	@echo '  make probe        report what the pad exposes (no daemon needed)'
	@echo '  make learn        print what each key sends; ctrl-c when done'
	@echo
	@echo 'installed from: $(REPO)'

build:
	cargo build --release

# config.toml is gitignored and yours; presets/ is tracked and shared. Created
# on first install only, so neither `make update` nor a `git pull` can ever
# tread on bindings you have edited.
$(CONFIG):
	@cp $(PRESETS)/$(PRESET).toml $@
	@echo '==> created config.toml from presets/$(PRESET).toml'

config: $(CONFIG)

presets:
	@echo 'presets:'
	@for p in $(PRESETS)/*.toml; do \
		n=$$(basename $$p .toml); \
		printf '  %-14s %s\n' "$$n" "$$(head -1 $$p | sed 's/^# *//')"; \
	done
	@echo
	@echo '  make preset-use NAME=<name>    adopt one (overwrites config.toml)'
	@echo '  make preset-save NAME=<name>   publish your config.toml as one'

# Overwrites config.toml, so keep a copy of anything you care about first --
# hence the confirmation.
preset-use:
	@test -n "$(NAME)" || { echo 'usage: make preset-use NAME=<name>   (make presets to list)'; exit 2; }
	@test -f $(PRESETS)/$(NAME).toml || { echo "no such preset: $(NAME)"; exit 2; }
	@test ! -f $(CONFIG) || { printf 'overwrite your config.toml with preset "$(NAME)"? [y/N] '; \
		read -r a; [ "$$a" = y ] || { echo aborted; exit 1; }; }
	@cp $(PRESETS)/$(NAME).toml $(CONFIG)
	@echo '==> config.toml is now presets/$(NAME).toml'
	@systemctl --user restart $(UNIT_NAME) 2>/dev/null || true

# The publish step, and deliberately a separate one: your live config changes
# every time you tweak a key or the web UI saves. A preset should be a thing you
# chose to share, not a file that drifts under you.
preset-save:
	@test -n "$(NAME)" || { echo 'usage: make preset-save NAME=<name>'; exit 2; }
	@test -f $(CONFIG) || { echo 'no config.toml to save'; exit 2; }
	@cp $(CONFIG) $(PRESETS)/$(NAME).toml
	@echo '==> saved config.toml to presets/$(NAME).toml -- commit it to publish'

# uaccess only attaches on a device event, so trigger one -- otherwise the rule
# does nothing until the pad is next replugged.
udev:
	@echo '==> installing $(UDEV) (needs sudo)'
	sudo install -m 0644 $(REPO)/udev/$(UDEV_RULE) $(UDEV)
	sudo udevadm control --reload-rules
	sudo udevadm trigger --subsystem-match=input --subsystem-match=hidraw

# The unit is generated, not shipped: it has to name an absolute path to the
# binary, and where you cloned this is not knowable in advance.
unit: build
	@mkdir -p $(UNIT_DIR)
	@sed -e 's|@BIN@|$(BIN)|g' -e 's|@REPO@|$(REPO)|g' \
		$(REPO)/systemd/$(UNIT_NAME).in > $(UNIT)
	@echo '==> wrote $(UNIT)'
	@systemctl --user daemon-reload

install: build config udev unit
	@systemctl --user enable --now $(UNIT_NAME)
	@echo
	@systemctl --user is-active --quiet $(UNIT_NAME) \
		&& echo '==> running. `make logs` to watch it, `make status` to check.' \
		|| { echo '==> FAILED to start:'; systemctl --user status $(UNIT_NAME) --no-pager -n 20; exit 1; }

update: build
	@systemctl --user restart $(UNIT_NAME)
	@echo '==> rebuilt and restarted'

# Leaves config.toml alone: it is yours, and a reinstall should not lose it.
uninstall:
	-@systemctl --user disable --now $(UNIT_NAME) 2>/dev/null
	@rm -f $(UNIT)
	@systemctl --user daemon-reload
	@echo '==> removing $(UDEV) (needs sudo)'
	-sudo rm -f $(UDEV)
	-sudo udevadm control --reload-rules
	@echo '==> removed. config.toml kept; target/ left for `cargo clean`.'

reinstall:
	@$(MAKE) uninstall
	@$(MAKE) install

status:
	@systemctl --user status $(UNIT_NAME) --no-pager || true

logs:
	@journalctl --user -u $(UNIT_NAME) -f

# Both grab the pad, so stop the daemon first or they will not get it.
probe: build
	@$(BIN) probe

learn: build
	@systemctl --user stop $(UNIT_NAME) 2>/dev/null || true
	-@$(BIN) learn
	@systemctl --user start $(UNIT_NAME) 2>/dev/null || true
