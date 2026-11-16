return {on_event=function(e,w,d)
local n=0; for i,x in ipairs({3,5,7}) do n=n+i*x end; return {data={sum=n,floor=math.floor(2.75)},commands={}}
end}
