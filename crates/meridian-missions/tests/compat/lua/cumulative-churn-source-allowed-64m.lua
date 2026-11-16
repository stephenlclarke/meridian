return {on_event=function(e,w,d)
d.counter=99; local x='x'; for i=1,13 do x=x..x end; local last=0; for i=1,6000 do local y=x..i; last=#y end; return {data={last=last},commands={}}
end}
