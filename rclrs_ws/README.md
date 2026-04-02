# rclrs workspace

Rclrs is the Rust 🦀 client for ROS2. It should be included in the 2026 release of ROS2 (Lyrical Luth). This workspace will be used for development until rclrs is released

ℹ️ More info available at the [ros2_rust](https://github.com/ros2-rust/ros2_rust/tree/main) source repo.

## Pixi dependencies install

❗ Make sure Pixi is installed on your system. If not, follow [these](https://pixi.prefix.dev/latest/installation/) steps ❗

First, install all the dependencies required to build rclrs. Those dependencies contain:

- The basic ROS2 Distro
- Colcon, colcon-cargo and colcon-ros-cargo to be able to build classical and rust ROS2 packages
- rust-src and rust, which might make sense to build a rust client library
- vcstool to import the rclrs repos

In order to install the corresponding environment, use the following commands:

```bash
cd rclrs_install_ws
pixi install
pixi shell
```

## Cloning the ros2 rust repository

Build the ros2 rust repository from its source repo (using the v0.7.0 release):

```bash
mkdir src
git clone -b v0.7.0 https://github.com/ros2-rust/ros2_rust.git src/ros2_rust
```

Once cloned, import the required repos for ros2_rust:

```bash
vcs import src < src/ros2_rust/ros2_rust_kilted.repos
# Don't build examples
touch src/ros2-rust/examples/COLCON_IGNORE
```

## Building ROS2 Rust

Now that everything is setup, build the library:

```bash
pixi run build
```