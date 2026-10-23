# The desktop's environment, for running Omarchy commands from SSH on the onn.
export HOME=/run/onn-probe/home XDG_RUNTIME_DIR=/run/user/0 XDG_CONFIG_HOME=/run/onn-probe/home/.config
export XDG_CACHE_HOME=/run/onn-probe/home/.cache XDG_DATA_DIRS=/usr/local/share:/usr/share
export OMARCHY_PATH=/usr/share/omarchy PATH=/usr/local/bin:/usr/share/omarchy/bin:/usr/bin:/bin LANG=C.UTF-8
for s in /run/user/0/wayland-*; do [ -S "$s" ] && export WAYLAND_DISPLAY=${s##*/}; done
for d in /run/user/0/hypr/*; do [ -d "$d" ] && export HYPRLAND_INSTANCE_SIGNATURE=${d##*/}; done
# The session bus belongs to the dbus-run-session that started Hyprland; read its address from there.
h=$(pidof Hyprland 2>/dev/null | cut -d" " -f1)
[ -n "$h" ] && export $(tr "\0" "\n" < /proc/$h/environ | grep -E "^DBUS_SESSION_BUS_ADDRESS=")
