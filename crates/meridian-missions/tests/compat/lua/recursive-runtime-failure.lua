return {on_event=function(e,w,d)
local function f() return 1+f() end; return f()
end}
