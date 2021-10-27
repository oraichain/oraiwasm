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
 
 #define __Nectar_ENV_STD
 #include "nectar.hpp"
 
 using namespace NectarCore::Global;
 using namespace NectarCore::Functions;
 using namespace NectarCore::JS;
 var __NJS_ENV = "std";
 var __NJS_PLATFORM = "darwin";
 
 #define __NJS_Create_Object() new NectarCore::Class::Object()
 #define __NJS_Create_Array(_arr) new NectarCore::Class::Array(_arr)
 #define __NJS_InitVar() NectarCore::VAR()
 #include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/lib/console.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/libperformance/perf.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/libregexp/regexp.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/lib/native_object.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/lib/math.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/lib/json.h"
#include "/Users/thanhtu/Projects/oraichain/oraiwasm/package/price/testcase_price_new/.nectar/uo5t1/libdate/date.h"
 
 var console;;var performance;;var Error;;var RegExp;;var Number;;var Object;;var Math;;var JSON;;var Array;;var Date;;var __NJS_Create_Object;;var __MODULE_gajh4k5jln;;var __MODULE_ych9e99z9b;;var _performance;;var __MODULE_7z3ymjsxnx;;var __MODULE_qbyauatbkz;;var _regexp;;var __MODULE_7s6gxofjaz;;var _Number;;var __MODULE_xy9v4rhqff;;var __MODULE_aqndtt5gh7;;var __MODULE_3e8hn1tm5u;;var __MODULE_75au6n0goc;;var __MODULE_bsy130mgf0;;var _date;;var __qk1j1;;var __adyra;

var window; // browser check

int main(int argc, char* argv[])
{
	var __NJS_ARGS = __NJS_Create_Array();
	
	for( int i = 0; i < argc; i++)
	{
		__NJS_ARGS[i] = argv[i];
	}

	try 
	{

		NectarCore::Type::function_t* __NJS_FN___6xylj8 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;
  var console;

   module = __NJS_Create_Object();

  ;
   console = __NJS_Create_Object();
  console["log"] = __Nectar_NATIVE_LOG_CONSOLE;
  ;
  module["exports"] = console;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_gajh4k5jln=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___6xylj8);

;

NectarCore::Type::function_t* __NJS_FN___ozxqll = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

   module = __NJS_Create_Object();

  ;

  NectarCore::Type::function_t* __NJS_FN___e78ug4 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
    __Nectar_THIS["timeOrigin"] = __Nectar_NATIVE_PERFORMANCE_NOW();

 __Nectar_THIS["now"] = NectarCore::VAR(NectarCore::Enum::Type::Function, new NectarCore::Type::function_t ([&](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{
      return (__Nectar_NATIVE_PERFORMANCE_NOW() - __Nectar_THIS["timeOrigin"]) * 1000;
    
return NectarCore::Global::undefined;}), __Nectar_THIS);;
   ;return NectarCore::Global::undefined;} );_performance=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___e78ug4);

  ;
  module["exports"] = __Nectar_NEW(_performance)();
  ;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_ych9e99z9b=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___ozxqll);

;

NectarCore::Type::function_t* __NJS_FN___ei98wx6m = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

  var _Error;

   module = __NJS_Create_Object();

   _Error = __NJS_Create_Object();
  ;
  module["exports"] = _Error;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_7z3ymjsxnx=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___ei98wx6m);

;

NectarCore::Type::function_t* __NJS_FN___5zje0o = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

   module = __NJS_Create_Object();

  ;

  NectarCore::Type::function_t* __NJS_FN___gb14kk = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {var _expression; if(__Nectar_VARLENGTH > 0) _expression = __Nectar_VARARGS[0];var  _flag; if(__Nectar_VARLENGTH > 1)  _flag = __Nectar_VARARGS[1];
    __Nectar_THIS["__Nectar_Internal_Expression"] = _expression;
    __Nectar_THIS["flag"] = _flag;
    __Nectar_THIS["test"] = __Nectar_RegExp_Test;
    __Nectar_THIS["exec"] = __Nectar_RegExp_Exec;
   ;return NectarCore::Global::undefined;} );_regexp=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___gb14kk);

  ;
  module["exports"] = _regexp;
  ;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_qbyauatbkz=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___5zje0o);

;

NectarCore::Type::function_t* __NJS_FN___mm7gnp = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

   module = __NJS_Create_Object();

  NectarCore::Type::function_t* __NJS_FN___24c61 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {var _arg; if(__Nectar_VARLENGTH > 0) _arg = __Nectar_VARARGS[0];
    if (_arg) {
      if (__Nectar_typeof(_arg) == __Nectar_InitVar("string")) {
        return parseInt(_arg);
      } else return _arg;
    }

    return 0;
   ;return NectarCore::Global::undefined;} );_Number=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___24c61);

  module["exports"] = _Number;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_7s6gxofjaz=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___mm7gnp);

;

NectarCore::Type::function_t* __NJS_FN___5suf66 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

  var _Object;

   module = __NJS_Create_Object();

  ;
   _Object = __NJS_Create_Object();
  _Object["keys"] = __Nectar_NATIVE_OBJECT_KEYS;
  _Object["freeze"] = __Nectar_NATIVE_OBJECT_FREEZE;
  ;
  module["exports"] = _Object;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_xy9v4rhqff=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___5suf66);

;

NectarCore::Type::function_t* __NJS_FN___2981qz = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

  var _Math;

   module = __NJS_Create_Object();

  ;
   _Math = __NJS_Create_Object();
  _Math["E"] = __Nectar_MATH_E;
  _Math["LN2"] = __Nectar_MATH_LN2;
  _Math["LOG2E"] = __Nectar_MATH_LOG2E;
  _Math["LOG10E"] = __Nectar_MATH_LOG10E;
  _Math["PI"] = __Nectar_MATH_PI;
  _Math["SQRT1_2"] = __Nectar_MATH_SQRT1_2;
  _Math["SQRT2"] = __Nectar_MATH_SQRT2;
  _Math["abs"] = __Nectar_MATH_ABS;
  _Math["acos"] = __Nectar_MATH_ACOS;
  _Math["acosh"] = __Nectar_MATH_ACOSH;
  _Math["asin"] = __Nectar_MATH_ASIN;
  _Math["asinh"] = __Nectar_MATH_ASINH;
  _Math["atan"] = __Nectar_MATH_ATAN;
  _Math["atanh"] = __Nectar_MATH_ATANH;
  _Math["atan2"] = __Nectar_MATH_ATAN2;
  _Math["cbrt"] = __Nectar_MATH_CBRT;
  _Math["ceil"] = __Nectar_MATH_CEIL;
  _Math["clz32"] = __Nectar_MATH_CLZ32;
  _Math["cos"] = __Nectar_MATH_COS;
  _Math["cosh"] = __Nectar_MATH_COSH;
  _Math["exp"] = __Nectar_MATH_EXP;
  _Math["expm1"] = __Nectar_MATH_EXPM1;
  _Math["floor"] = __Nectar_MATH_FLOOR;
  _Math["fround"] = __Nectar_MATH_FROUND;
  _Math["hypot"] = __Nectar_MATH_HYPOT;
  _Math["imul"] = __Nectar_MATH_IMUL;
  _Math["log"] = __Nectar_MATH_LOG;
  _Math["log1p"] = __Nectar_MATH_LOG1P;
  _Math["log10"] = __Nectar_MATH_LOG10;
  _Math["log2"] = __Nectar_MATH_LOG2;
  _Math["max"] = __Nectar_MATH_MAX;
  _Math["min"] = __Nectar_MATH_MIN;
  _Math["pow"] = __Nectar_MATH_POW;
  _Math["random"] = __Nectar_MATH_RANDOM;
  _Math["round"] = __Nectar_MATH_ROUND;
  _Math["sign"] = __Nectar_MATH_SIGN;
  _Math["sin"] = __Nectar_MATH_SIN;
  _Math["sinh"] = __Nectar_MATH_SINH;
  _Math["sqrt"] = __Nectar_MATH_SQRT;
  _Math["tan"] = __Nectar_MATH_TAN;
  _Math["tanh"] = __Nectar_MATH_TANH;
  _Math["trunc"] = __Nectar_MATH_TRUNC;

   __qk1j1 = NectarCore::VAR(NectarCore::Enum::Type::Function, new NectarCore::Type::function_t ([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{
                      

    return __Nectar_InitVar("[object Math]");
  
return NectarCore::Global::undefined;}), __Nectar_THIS);;

  _Math["toString"] = __qk1j1;
  ;
  module["exports"] = _Math;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_aqndtt5gh7=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___2981qz);

;

NectarCore::Type::function_t* __NJS_FN___4xnyg8 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

  var _JSON;

   module = __NJS_Create_Object();

  ;
   _JSON = __NJS_Create_Object();
  _JSON["parse"] = __Nectar_JSON_PARSE;
  _JSON["stringify"] = __Nectar_JSON_STRINGIFY;
  ;
  module["exports"] = _JSON;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_3e8hn1tm5u=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___4xnyg8);

;

NectarCore::Type::function_t* __NJS_FN___d5b6b2 = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

  var _Array;

   module = __NJS_Create_Object();

   _Array = __NJS_Create_Object();

  var __isurxw = __NJS_Create_Object();

   __adyra = NectarCore::VAR(NectarCore::Enum::Type::Function, new NectarCore::Type::function_t ([](var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH) -> NectarCore::VAR{
                      

    console["log"](__Nectar_THIS);
  
return NectarCore::Global::undefined;}), __Nectar_THIS);;

  __isurxw["slice"] = __adyra;
  _Array["prototype"] = __isurxw;
  ;
  module["exports"] = _Array;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_75au6n0goc=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___d5b6b2);

;

NectarCore::Type::function_t* __NJS_FN___29cwil = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {
  var module;

   module = __NJS_Create_Object();

  ;

  NectarCore::Type::function_t* __NJS_FN___1hosmd = new NectarCore::Type::function_t([&]( var __Nectar_THIS, NectarCore::VAR* __Nectar_VARARGS, int __Nectar_VARLENGTH ) -> NectarCore::VAR {var _date; if(__Nectar_VARLENGTH > 0) _date = __Nectar_VARARGS[0];
    __Nectar_THIS["__Nectar_Internal_Date"] = _date;
   ;return NectarCore::Global::undefined;} );_date=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___1hosmd);

  ;
  _date["now"] = __Nectar_NATIVE_DATE_NOW;
  module["exports"] = _date;
  ;
  return module["exports"];
 ;return NectarCore::Global::undefined;} );__MODULE_bsy130mgf0=NectarCore::VAR(NectarCore::Enum::Type::Function, __NJS_FN___29cwil);

;
		
		#ifdef __Nectar_INIT_RAND_SEED
		srand (time(NULL));
		#endif
		{
			console = __MODULE_gajh4k5jln();
performance = __MODULE_ych9e99z9b();
Error = __MODULE_7z3ymjsxnx();
RegExp = __MODULE_qbyauatbkz();
Number = __MODULE_7s6gxofjaz();
Object = __MODULE_xy9v4rhqff();
Math = __MODULE_aqndtt5gh7();
JSON = __MODULE_3e8hn1tm5u();
Array = __MODULE_75au6n0goc();
Date = __MODULE_bsy130mgf0();
console["log"]("running script");

			NectarCore::Event::Loop();
		}
		
	}
	catch(NectarCore::VAR __Nectar_Global_Exception)
	{
		__Nectar_Log_Console(__Nectar_Global_Exception);
		exit(1);
	}
	return 0;
}
