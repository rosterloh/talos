# Introduction

Talos is a terminal-native tool for observing and interacting with ROS 2
systems.

The main idea is to separate the developer workstation from the ROS 2 runtime.
The robot runs `talos-agent`, which talks to ROS 2 through `rclrs`. Developer
machines run `talos-tui` or `talos-cli`, which connect to the agent over Unix
domain sockets for local use or QUIC for remote use.

```text
Developer machine                      Target device

talos-tui                              talos-agent
talos-cli          UDS or QUIC         rclrs
no ROS 2 needed  <------------->       ROS 2 graph
```

Only the agent depends on ROS 2. The shared protocol, CLI, and TUI build and run
without a ROS 2 installation.

## What Talos Provides

- Topic observation with live message data rendered as generic trees.
- ROS 2 node discovery and graph inspection.
- A filterable `/rosout` log view.
- URDF-aware joint state display and joint command publishing.
- A small CLI for scripts and one-shot checks.

## Current Scope

Talos currently focuses on observation, basic joint control, and terminal
workflows. It does not try to replace RViz, provide full ROS 2 service/action
proxying, or expose arbitrary dynamic message support beyond the compiled-in
message conversions in the agent.

## Documentation Status

This book is the canonical project documentation. Keep it aligned with current
behavior and use the design history pages for context on important choices.
