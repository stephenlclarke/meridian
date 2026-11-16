local __mission_factories={}
local __mission_loaded={}
local __mission_loading={}
local require
require=function(name)
  if type(name)~='string' then error('Module name must be a declared string') end
  if __mission_loading[name] then error('Cyclic mission module: '..name) end
  if __mission_loaded[name] then return __mission_loaded[name] end
  local factory=__mission_factories[name]
  if not factory then error('Undeclared mission module: '..name) end
  __mission_loading[name]=true
  local result=factory()
  if type(result)~='table' then error('Mission module must return a table: '..name) end
  __mission_loaded[name]=result
  __mission_loading[name]=nil
  return result
end

-- scripts/a.lua
__mission_factories['a']=function()
return {}
end

return (function()
local x=require('missing'); return {}
end)()
