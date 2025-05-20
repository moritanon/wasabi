#!/bin/bash -e
PROJ_ROOT="$(dirname $(dirname ${BASH_SOURCE:-$0}))"
cd "${PROJ_ROOT}"

PATH_TO_EFI="$1"
rm -rf mnt
mkdir -p mnt/EFI/BOOT/
cp "${PATH_TO_EFI}" mnt/EFI/BOOT/BOOTX64.EFI
set +e
mkdir -p log
# /mnt/c/Program\ Files/uvnc\ bvba/UltraVNC/vncviewer.exe 127.0.0.1:5900&
/mnt/c/Program\ Files/qemu/qemu-system-x86_64.exe  \
  -m 4G \
  -bios third_party/ovmf/RELEASEX64_OVMF.fd \
  -machine q35 \
  -monitor telnet:0.0.0.0:2345,server,nowait,logfile=log/qemu_monitor.txt \
  -drive format=raw,file=fat:rw:mnt \
  -chardev stdio,id=char_com1,mux=on,logfile=log/com1.txt \
  -serial chardev:char_com1 \
  -device qemu-xhci \
  -device usb-kbd \
  -device isa-debug-exit,iobase=0xf4,iosize=0x01
RETCODE=$?
set -e
if [ $RETCODE -eq 0 ]; then
  exit 0;
elif [ $RETCODE -eq 3 ]; then
  printf "\nPASS\n";
  exit 0;
else 
  printf "\nFAIL: QEMU returned $RETCODE\n";
  exit 1
fi