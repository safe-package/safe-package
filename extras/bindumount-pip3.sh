#!/usr/bin/env bash

jail=/cellblock/piptest


# /proc can be bind-mounted and unmounted per normal.
umount $jail/proc
rmdir $jail/proc


# /dev and /sys need to be recursively mounted and unmounted, and "slaved" to
# their "master" counterparts.
for d in dev sys   
do
	if [ -d $jail/$d ]
	then
		umount -R $jail/$d
		rmdir $jail/$d
	fi
done


# Everything else can be bind-mounted and unmounted as usual. We'll unmount
# recursively to avoid unnessary errors.
mount | grep $jail | cut -d' ' -f3 | while read m
do
	umount -R $m
done

