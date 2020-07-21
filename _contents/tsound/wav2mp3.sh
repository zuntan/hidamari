#!/bin/bash

LAME=lame

for x in *wav
do
	echo ${LAME} -b 320 -V 2 $x ${x%.wav}.mp3
	${LAME} -b 320 -V 2 $x ${x%.wav}.mp3
done
