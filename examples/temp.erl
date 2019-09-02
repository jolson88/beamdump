-module(temp).
-export([format_temps/1]).

% String constants
-define(GREETING1, "Hello, ").
-define(GREETING2, "World!").

format_temps([]) ->
	ok;
format_temps([City | Rest]) ->
	print_temp(convert_to_celsius(City)),
	format_temps(Rest).

convert_to_celsius({Name, {c, Temp}}) ->
	{Name, {c, Temp}};
convert_to_celsius({Name, {f, Temp}}) ->
	{Name, {c, (Temp - 32) * 5 / 9}}.

print_temp({Name, {c, Temp}}) ->
   	io:fwrite("~p~p~n",[?GREETING1, ?GREETING2]),
	io:format("~-15w ~w c~n", [Name, Temp]).
