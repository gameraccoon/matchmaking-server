#!/bin/bash

pushd $1
	$2 ${@:3:99} >/dev/null 2>/dev/null &
popd
