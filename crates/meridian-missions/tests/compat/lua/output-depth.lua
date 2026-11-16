return {on_event=function(e,w,d)
local x={}; for i=1,32 do x={child=x} end; return {data=x,commands={}}
end}
