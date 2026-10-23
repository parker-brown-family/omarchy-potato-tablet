# onn 10.1" Tablet Pro: the remaining hardware on the stock 4.19 kernel

Researched 2026-10-23. The onn's kernel source is unpublished, so code facts come from the closest public sibling: the Lenovo Tab M8 (3rd gen) / Moto Tab G20 kernel, same MediaTek BSP (Linux 4.19.127, `CONFIG_MTK_PLATFORM="mt6765"`, board `tb8768p1`). Check every node on the tablet before relying on it; start each section with `zcat /proc/config.gz` and `dmesg`.

## 1. Audio (speakers)

**How this BSP routes audio.** The ALSA card is `mt-snd-card`, MediaTek's older "common_int" ASoC driver, paired with the MT6357 PMIC codec. MediaTek's Android audio HAL does not use `mixer_paths.xml`. It reads `/vendor/etc/audio_device.xml` (sometimes `/vendor/etc/audio_param/audio_device.xml`) and sets mixer controls by name. The PCM driver does the digital routing itself: opening `MultiMedia1_PLayback` (DL1) connects memory to the I2S DAC. Userspace only has to switch on the analog and amplifier stage:

- `Speaker_Amp_Switch` On: powers the MT6357 DAC and line-out, which feeds an external class-D amp.
- `Ext_Speaker_Amp_Switch` On: pulses the `extamp-pullhigh` pinctrl GPIO three times, the one-wire mode select used by AW87xx-style amps.
- Gain: `Lineout_PGAL_GAIN` and `Lineout_PGAR_GAIN`.
- A board with an I2C/I2S smart amp instead adds its own controls. The Lenovo M8 has two Awinic AW8838 (`aw8838_l_speaker_switch`, `aw8838_r_speaker_switch`), and its stream may go through `I2S0DL1_PLayback` with `Audio_I2S0dl1_hd_Switch`.
- Test with no stream: the AFE sine generator, `Audio_SineGen_Switch` (e.g. `I5I6`, `O3O4`).

Try first:

```sh
cat /proc/asound/cards /proc/asound/pcm
amixer -c0 controls | grep -iE 'spk|speaker|amp|aw8|lineout'
grep -n -A6 -iE 'speaker_output|ext_speaker' /vendor/etc/audio_device.xml /vendor/etc/audio_param/audio_device.xml
grep -rlsE 'aw87|aw88|fs16|sia81|tfa98|extamp|smartpa' /proc/device-tree | head
amixer -c0 cset name='Speaker_Amp_Switch' On
amixer -c0 cset name='Ext_Speaker_Amp_Switch' On
speaker-test -D hw:0,N -c2 -r48000 -F S16_LE -t sine
```

`N` is the index of `MultiMedia1_PLayback` in `/proc/asound/pcm`. The onn's own `speaker_output` / `ext_speaker_output` path in its `audio_device.xml` is the ground truth; copy it exactly.

**PipeWire.** `mt-snd-card` has no UCM profile, so automatic probing walks about twenty PCMs, the likely cause of the incomplete routing (unconfirmed). Disable WirePlumber's ALSA monitor for this card, define one static sink, and set the controls from a boot script or a UCM2 `EnableSequence`:

```
context.objects = [ { factory = adapter args = { factory.name = api.alsa.pcm.sink node.name = onn-speaker media.class = Audio/Sink api.alsa.path = "hw:0,N" audio.format = S16LE audio.rate = 48000 audio.channels = 2 } } ]
```

**Confirm:** you hear a tone, and `cat /proc/asound/card0/pcmNp/sub0/status` shows `RUNNING`.

## 2. Battery level

**Why capacity reads −1.** In `mtk_battery.c`, `bat_capacity` starts at −1. At probe the driver chooses one of two modes. It picks *daemon mode* when its netlink socket (`NETLINK_FGD`, 26) exists and the boot mode is not recovery (bootmode 2). Otherwise it runs the gauge algorithm inside the kernel. In daemon mode, capacity stays −1 until the vendor `fuelgauged` connects and pushes a state of charge. Android starts that daemon from `fuelgauged_init.rc` (`service fuelgauged /vendor/bin/fuelgauged`, class core, user system), with `fuelgauged_nvram` beside it. healthd computes nothing; it only reads sysfs.

Try first:

```sh
dmesg | grep -E 'daemon mode DONE|kernel mode DONE|Vbat='
cat /sys/class/power_supply/battery/voltage_now /sys/class/power_supply/battery/current_now
ls /sys/class/power_supply/mtk-gauge/
cat /sys/class/power_supply/mtk-gauge/bat_vol_get /sys/class/power_supply/mtk-gauge/rtc_ui_soc
```

`voltage_now` (µV) and `current_now` read the gauge directly and work without the daemon.

Fix it in this order:

1. **Run `/vendor/bin/fuelgauged`** the same way as `wmt_loader`: the APEX linker plus the property shim. Add a shim for `__android_log_print` so you can see what it does. Whatever else it needs (NVRAM, more properties) is unconfirmed.
2. **Fallback, with no daemon:** estimate the charge from voltage and write it into the driver, in units of 0.01 %: `echo $((pct*100)) > /sys/class/power_supply/battery/uisoc`. The driver's `uisoc_set` stores `(val+50)/100` and copies it to `capacity`. The OCV table is in the device tree: `find /proc/device-tree -name 'battery0_profile_t*'` returns triplets of depth of discharge (0.01 %), voltage (0.1 mV) and resistance. `rtc_ui_soc` holds the last level stored in the PMIC, which gives a starting value.

**Confirm:** `capacity` is no longer −1.

## 3. Accelerometer and rotation

On 4.19 `sensors-1.0`, the device class is `sensor`, not `misc`. The nodes are `/dev/m_acc_misc` and `/sys/class/sensor/m_acc_misc/{accactive,accbatch,accflush,acccali,accdevnum}`. The Lenovo M8 reads its accelerometer through the SCP sensor-hub coprocessor (`CONFIG_MTK_ACCELHUB`), whose firmware the stock bootloader loads. The onn may use a direct-I2C driver instead; the interface is the same.

```sh
zcat /proc/config.gz | grep -E 'ACCEL|HUB|SCP'
echo 0,0,66667000,0 > /sys/class/sensor/m_acc_misc/accbatch   # handle,flag,delay_ns,latency_ns
echo 1 > /sys/class/sensor/m_acc_misc/accactive
cat /sys/bus/platform/drivers/gsensor/sensordata               # hub driver prints "%04x %04x %04x %04x" = x y z status
```

**Event stream.** Call `poll()` on `/dev/m_acc_misc`, then read multiples of 36 bytes. Each record is packed: int64 timestamp, then int8 handle, flush_action, status and reserved, then int32 `word[6]`, with x, y, z in `word[0..2]`. `read()` returns 0 when the queue is empty, so always poll first. The units are probably m/s² × 1000 (`GRAVITY_EARTH_1000` is 9807). Confirm by laying the tablet flat, where one axis should read about 9807.

**Feeding the desktop.** iio-sensor-proxy accepts an input device tagged `ID_INPUT_ACCELEROMETER`, so a uinput bridge is possible. That tag comes from udev rules, though, and libudev-zero may not provide it. A small daemon is simpler and is the recommendation. It polls `sensordata` twice a second and applies hysteresis. When the orientation changes it runs `hyprctl keyword monitor <name>,preferred,auto,1,transform,N` and `hyprctl keyword input:touchdevice:transform N`, where `<name>` comes from `hyprctl monitors`.

**Confirm:** `sensordata` changes as you turn the tablet. If it stays at zero, look at `dmesg | grep -iE 'accel|scp'` (the SCP may not be ready).

## 4. Suspend and screen off

The bootloader and ATF load the SPM sleep firmware, and the kernel enters sleep through SMC calls, so suspend needs no Android component. Android supplies only the policy: its system_suspend service writes `wakeup_count`, then `mem`.

**Why the serial shell hung.** Reading `/sys/power/wakeup_count` blocks while any wakeup source is active (kernel ABI documentation). MediaTek's USB driver holds `USB suspend lock` the whole time a host is connected. So with the serial cable in, opportunistic suspend can never happen. A plain `echo mem > /sys/power/state`, without a `wakeup_count` write, does not check wakeup sources. It will suspend, and it will drop the USB serial link.

```sh
cat /sys/power/state /sys/power/mem_sleep; ls /sys/power/autosleep
mount -t debugfs none /sys/kernel/debug; awk 'NR>1 && $6>0' /sys/kernel/debug/wakeup_sources
echo 4 > /sys/class/graphics/fb0/blank; echo 0 > /sys/class/leds/lcd-backlight/brightness
echo +30 > /sys/class/rtc/rtc0/wakealarm; echo mem > /sys/power/state
dmesg | grep -E 'wake up by|PM: suspend'
echo 0 > /sys/class/graphics/fb0/blank
```

`fb0/blank` = 4 is `FB_BLANK_POWERDOWN`, which `mtkfb` maps to its display early-suspend; 0 brings the panel back.

**Traps:**

- Test with the RTC alarm, so the tablet comes back on its own.
- Check Wi-Fi (WMT) after resume.
- Hyprland on fbdev may need a full redraw after unblank (unconfirmed).
- The power key should wake it through the MT6357 PMIC keypad (likely, unconfirmed).

## 5. GPU

**There is no open path today.** Mesa's PowerVR Vulkan driver shipped in Mesa 25.3 (October 2025) and supports AXE-1-16M, BXM-4-64 and BXS-4-64. Mesa lists the GE8300 (Series 8XE) only as "unsupported, not actively developed", and does not list the GE8320 at all. The driver also needs the upstream `drm/imagination` kernel driver (Linux 6.8 and later). Current mainline accepts only those three cores; anything else sits behind `exp_hw_support`. There is no usable mainline kernel for MT6765/MT8768 either, only a basic 2020 device tree; postmarketOS lists the 7" onn Gen 4 (mt6765) as not booting (from search results; the wiki is behind a bot wall).

**The likely cause of your `ENODEV`.** The 4.19 kernel carries the vendor DDK `rgx doma m1.11ED5425693`. Its DRM device initializes on the first open. It loads `rgx.fw.<BVNC>`, then `rgx.fw`, through `request_firmware`, and MediaTek's search path starts at `/vendor/firmware`. If every load fails, open returns `-ENODEV` and the device is marked BAD, so every later open also fails until a reboot.

```sh
dmesg | grep -iE 'RGX Firmware|PVR'
```

Look for `All RGX Firmware image loads failed for 'rgx.fw.B.V.N.C'`, which also tells you the BVNC. The lookup is most likely resolved inside the root of whichever process opens the device (verify), so the Arch chroot needs to see the firmware. Bind-mount `/vendor` into the chroot, or copy `/vendor/firmware/rgx.fw` into its `/lib/firmware`, then reboot.

**Even with the firmware loaded,** only the vendor's bionic GLES (`/vendor/lib64/egl/*mtk*`) can drive it, through libhybris, which needs gralloc and the Android binder/HAL stack (Halium-style). Droidian composites through a hwcomposer backend in wlroots/phoc; Hyprland's Aquamarine has none. No report found of libhybris on a GE8320. Treat it as research and stay on llvmpipe.

## Sources

Sibling 4.19.127 kernel (Lenovo Tab M8 3rd gen / Moto Tab G20), files read:

- https://github.com/geevarghesest/android_kernel_mt8768t
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/arch/arm/configs/tb8768p1_bsp_defconfig
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/sound/soc/mediatek/codec/mt6357/mtk-soc-codec-6357.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/sound/soc/mediatek/common_int/mtk-auddrv-def.h
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/sound/soc/mediatek/common_int/mtk-soc-pcm-dl1.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/sound/soc/mediatek/common_int/mtk-soc-pcm-routing.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/power/supply/mtk_battery.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/power/supply/mt6357-gauge.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/sensors-1.0/accelerometer/accel.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/sensors-1.0/accelerometer/accelhub/accelhub.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/sensors-1.0/hwmon/include/sensor_event.h
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/usb20/mt6765/usb20.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/video/mt6765/videox/mtkfb.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/misc/mediatek/base/power/spm/mt6765/mtk_spm_internal.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/gpu/mediatek/gpu_rgx/m1.11ED5425693/services/server/env/linux/module_common.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/gpu/mediatek/gpu_rgx/m1.11ED5425693/services/server/devices/rgx/rgxinit.c
- https://github.com/geevarghesest/android_kernel_mt8768t/blob/main/drivers/base/firmware_loader/main.c

Android-side configuration:

- https://github.com/maximus-sallam/android_device_mediatek/blob/main/mt6739/audio_device.xml
- https://github.com/LosSantosPro/android_device_lenovo_TB375FC/blob/lineage-23.2/rootdir/etc/init/fuelgauged_init.rc
- https://github.com/crdroidandroid/android_device_xiaomi_blossom/blob/17.0/rootdir/etc/init.mt6765.rc
- https://github.com/lenovo-mt6765/proprietary_vendor_lenovo_mt6765-common/blob/lineage-20/mt6765-common-vendor.mk

Linux and GPU:

- https://github.com/torvalds/linux/blob/master/Documentation/ABI/testing/sysfs-power
- https://github.com/torvalds/linux/blob/master/drivers/gpu/drm/imagination/pvr_device.c
- https://docs.mesa3d.org/drivers/powervr.html
- https://www.phoronix.com/news/PowerVR-Mesa-More-GPUs
- https://developer.imaginationtech.com/solutions/open-source-gpu-driver/
- https://lkml.iu.edu/hypermail/linux/kernel/2002.0/04829.html
- https://wiki.postmarketos.org/wiki/Onn_Tablet_Gen_4_(onn-obsidian)
- https://github.com/droidian-images/droidian/releases
- https://github.com/jwinarske/drm-cxx/blob/main/docs/hardware.md

Desktop side, and the device itself:

- https://gitlab.freedesktop.org/hadess/iio-sensor-proxy/-/blob/master/data/80-iio-sensor-proxy.rules
- https://www.hovatek.com/forum/printthread.php?tid=44660
