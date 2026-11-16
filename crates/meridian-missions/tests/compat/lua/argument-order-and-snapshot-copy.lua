return {on_event=function(e,w,d)
w.credits=w.credits+5; return {data={event=e.kind,credit=w.credits,old=d.old},commands={}}
end}
