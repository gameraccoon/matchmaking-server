#!/bin/bash

pushd $1
	$2 ${@:3:99} &
popd
