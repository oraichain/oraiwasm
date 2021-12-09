/*
 * This file is part of NectarJS
 * Copyright (c) 2017 - 2020 Adrien THIERRY
 * http://nectarjs.com - https://seraum.com/
 *
 * sources : https://github.com/nectarjs/nectarjs
 * 
 * NectarJS is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * NectarJS is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with NectarJS.  If not, see <http://www.gnu.org/licenses/>.
 *
 */
 
#include <regex>

NectarCore::Type::function_t* __NJS_FN___6wo5wk = new NectarCore::Type::function_t([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{var _search; if(__Nectar_VARLENGTH > 0) _search = __Nectar_VARARGS[0];
 	if( std::regex_match ( (std::string)_search, std::regex((std::string)__Nectar_THIS["__Nectar_Internal_Expression"], std::regex::ECMAScript) ) ) return __Nectar_Boolean_TRUE;
	else return __Nectar_Boolean_FALSE;
;return NectarCore::Global::undefined;});var __Nectar_RegExp_Test=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___6wo5wk);

NectarCore::Type::function_t* __NJS_FN___wuzgd4 = new NectarCore::Type::function_t([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{var _search; if(__Nectar_VARLENGTH > 0) _search = __Nectar_VARARGS[0];
	var _res = __NJS_Create_Array();
	std::string s = (std::string)_search;
	std::smatch m;
 	while(std::regex_search ( s, m, std::regex((std::string)__Nectar_THIS["__Nectar_Internal_Expression"], std::regex::ECMAScript) ))
	{
		int i = 0;
		for(auto x:m)
		{
			_res[i] = (std::string)x;
			i++;
		}
		_res["index"] = (double)m.position();
		_res["input"] = _search;
		_res["groups"] = NectarCore::Global::undefined;
		
		s = m.suffix().str();
	}
	return _res;
;return NectarCore::Global::undefined;});var __Nectar_RegExp_Exec=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___wuzgd4);

/*
NectarCore::Type::function_t* __NJS_FN___e5psj = new NectarCore::Type::function_t([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{var _search; if(__Nectar_VARLENGTH > 0) _search = __Nectar_VARARGS[0];var  _re; if(__Nectar_VARLENGTH > 1)  _re = __Nectar_VARARGS[1];
	var _res = __NJS_Create_Array();
	std::string s = (string)_search;
	std::smatch m;
 	while(std::regex_search ( s, m, std::regex((string)_re, std::regex::ECMAScript) ))
	{
		for(auto x:m) ((NectarCore::Class::Array*)_res.data.ptr)->value.push_back((string)x);
		s = m.suffix().str();
	}
	return _res;
;return NectarCore::Global::undefined;});var __Nectar_RegExp_StringMatch=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___e5psj);
*/