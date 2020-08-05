#!/bin/bash

LAME=lame

for x in *wav
do
	echo ${LAME} --cbr -b 192 -h $x ${x%.wav}.mp3
	${LAME} --cbr -b 192 -h $x ${x%.wav}.mp3
done
