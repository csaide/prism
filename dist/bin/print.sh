#!/usr/bin/env bash

# (c) Copyright 2022 Christian Saide
# SPDX-License-Identifier: GPL-3.0-or-later

if [[ ${TERM} == "dumb" ]]; then
    printf "= %s =" "${1}"
else
    COLUMNS=$(tput cols)
    LEN=$(echo "${1}" | wc -c)

    #printf "=%.s" $(seq 1 $(( (${COLUMNS} / 2) - ((${LEN} / 2) + 1) )))
    printf "\e[1m\e[92m[%s]\e[0m " "${1}"
    printf "=%.s" $(seq 1 $(( ${COLUMNS} - (${LEN} + 2) )))
    printf "\n"
fi
