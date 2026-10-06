# KeyRC X11 compositor

`keyrc-compositor` is a small XComposite manager for Openbox and other X11
window managers without a compositor. It is derived from X.Org's `xcompmgr`
and adds a live blur pass for windows marked with `_KEYRC_LIQUID_GLASS`.

Only one X11 compositor may own `_NET_WM_CM_S0`. KeyRC starts this helper when
liquid glass is enabled and no other compositor is active. It stops the helper
when KeyRC exits. If Picom, KWin, or another compositor is already running,
KeyRC leaves it untouched and uses its built-in fallback renderer.

The upstream license is in [COPYING](COPYING).
