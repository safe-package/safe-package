#!/usr/bin/env bash

set -e

# This script mocks up the bind-mount functionality that will eventually
# be implemented in rust. Prototyping in bash first makes it easy to 
# test, debug, validate, and demo.

jail=/cellblock/piptest
if [ ! -d $jail ]
then
	mkdir -p $jail
fi


# /proc can be bind-mounted and unmounted per normal.
if [ ! -d $jail/proc ]
then
	mkdir $jail/proc
fi
mount --bind /proc $jail/proc


# /dev and /sys need to be recursively mounted and unmounted, and "slaved" to 
# their "master" counterparts.
for d in dev sys
do
	if [ ! -d $jail/$d ]
	then
		mkdir $jail/$d
	fi
	mount --rbind --make-rslave /$d $jail/$d/
done


# Everything else can be bind-mounted and unmounted as usual. We'll unmount
# recursively to avoid unnessary errors.
#		"local: /bin mountpoint: /bin opts: ro",

cat $HOME/.safe-package/config.json | \
	grep mountpoint | \
	cut -d'"' -f2 | \
	cut -d' ' -f2,4,6 | \
	while read path mount opts
	do
		if [ -z "$path" ]
		then
			"empty line, continuing"
			continue
		fi

		mkdir -p ${jail}${path}
		if [ -z "$opts" ]
		then
			mount --bind $path ${jail}${mount}
		else
			mount --bind -o "$opts" $path ${jail}${mount}
		fi
	done


