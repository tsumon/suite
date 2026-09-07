// Copyright (C) TranslucentTB contributors
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Copied from TranslucentTB Common/redefgetcurrenttime.h
// https://github.com/TranslucentTB/TranslucentTB
// commit d4636e439865df0a1a1419db408e055740ce5c74

#ifdef GET_CURRENT_TIME_UNDEFINED
# pragma pop_macro("GetCurrentTime")
# undef GET_CURRENT_TIME_UNDEFINED
#else
# error "GetCurrentTime has not been undefined"
#endif
