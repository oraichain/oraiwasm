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

NectarCore::Type::function_t* __NJS_FN___vp06ze = new NectarCore::Type::function_t([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{var _obj; if(__Nectar_VARLENGTH > 0) _obj = __Nectar_VARARGS[0];
	return __Nectar_Object_Keys(_obj);
;return NectarCore::Global::undefined;});var __Nectar_NATIVE_OBJECT_KEYS=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___vp06ze);;

NectarCore::Type::function_t* __NJS_FN___zh5jx = new NectarCore::Type::function_t([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{var _obj; if(__Nectar_VARLENGTH > 0) _obj = __Nectar_VARARGS[0];
	if(_obj)
	{
		if(_obj.type == NectarCore::Enum::Type::Object)
		{
			((NectarCore::Class::Object*)_obj.data.ptr)->property.set(0, 1);
			return _obj;
		}
	}
;return NectarCore::Global::undefined;});var __Nectar_NATIVE_OBJECT_FREEZE=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___zh5jx);;
