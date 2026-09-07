// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copied from TranslucentTB Common/undefgetcurrenttime.h
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74

#ifndef GET_CURRENT_TIME_UNDEFINED
# ifdef GetCurrentTime
#  pragma push_macro("GetCurrentTime")
#  undef GetCurrentTime
#  define GET_CURRENT_TIME_UNDEFINED
# else
#  error "GetCurrentTime is not defined"
# endif
#else
# error "GetCurrentTime has already been undefined"
#endif
