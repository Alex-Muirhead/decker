# Decker

Generates starting selections for games of Dominion.

Usage:
======
See `./decker --help` for options.

Implementaion goals:
====================
+ For a fixed seed and other commandline parameters the tool should be
 deterministic.
+ It should not use tools which can't be replicated reliably in other language 
 environments.

Naming style(c++):
==================
+ Files which describe a single class are named for that class (initial 
 capital camel-case).
+ Files which describe multiple-classes (or no classes) are named all 
 lower case.
+ Classes are named initial capital camel-case.
+ Typedefs are named like classes.
+ Free functions and all class members (both methods and variables) are named 
 with initial lower camel-case.
+ Local vars are named the same way as member variables

Contributions:
==============
I'll take bug reports, but if you want specific code merged, please fork instead.

License:
========
Copyright 2020 Joel Fenwick

+ Source code licensed under Apache 2.0 license.
+ Data files and tests are licensed under the MIT license.

Apache 2.0 blurb
----------------
>   Licensed under the Apache License, Version 2.0 (the "License");
>   you may not use this file except in compliance with the License.
>   You may obtain a copy of the License at

>     http://www.apache.org/licenses/LICENSE-2.0

>   Unless required by applicable law or agreed to in writing, software
>   distributed under the License is distributed on an "AS IS" BASIS,
>   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
>   See the License for the specific language governing permissions and
>   limitations under the License.

MIT (Expat) blurb
-----------------
>Permission is hereby granted, free of charge, to any person obtaining a copy 
>of this software and associated documentation files (the "Software"), to 
>deal in the Software without restriction, including without limitation the 
>rights to use, copy, modify, merge, publish, distribute, sublicense, and/or 
>sell copies of the Software, and to permit persons to whom the Software is 
>furnished to do so, subject to the following conditions:

>The above copyright notice and this permission notice shall be included in 
>all copies or substantial portions of the Software.

>THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR 
>IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
>FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE 
>AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
>LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, 
>OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN 
>THE SOFTWARE.

Legals
======
Dominion is designed by Donald X. Vaccarino and is published by RioGrande Games.

All related IP, trademarks etc belong to their owners;
ie not me (with the exception of my copyright in the sourcecode of 
this project).

