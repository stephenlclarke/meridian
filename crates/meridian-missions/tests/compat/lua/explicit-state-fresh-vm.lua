local hidden=0
return {on_event=function(e,w,d)
 hidden=hidden+1
 d.counter=(d.counter or 0)+hidden
 return {data=d,commands={{op='credits',amount=w.credits*2}}}
end}
