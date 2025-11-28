# PXE 测试文件目录

此目录用于存放 PXE 启动测试所需的文件。

## 文件说明

- `pxelinux.0`: Legacy BIOS PXE 启动文件（从 syslinux 获取）
- `bootx64.efi`: UEFI PXE 启动文件（从 syslinux 或 iPXE 获取）
- `ldlinux.c32`: syslinux 核心库文件
- `pxelinux.cfg/default`: PXE 启动菜单配置文件

## 准备文件

运行测试脚本时，`prepare_pxe_files.sh` 会自动准备这些文件。如果自动准备失败，可以手动准备：

1. **从 syslinux 包获取**:
   ```bash
   # Ubuntu/Debian
   sudo apt-get install syslinux-common
   cp /usr/lib/syslinux/modules/bios/pxelinux.0 tests/fixtures/
   cp /usr/lib/syslinux/modules/bios/ldlinux.c32 tests/fixtures/
   cp /usr/lib/syslinux/efi64/efi/syslinux.efi tests/fixtures/bootx64.efi
   ```

2. **从网络下载**:
   - syslinux: https://mirrors.edge.kernel.org/pub/linux/utils/boot/syslinux/
   - iPXE: https://ipxe.org/download

## 测试镜像（可选）

如果需要测试 HTTP 文件服务，可以添加以下文件：

- `vmlinuz`: Linux 内核文件
- `initrd.img`: 初始 RAM 磁盘文件

这些文件可以从 Linux 发行版的 PXE 安装镜像中获取。

