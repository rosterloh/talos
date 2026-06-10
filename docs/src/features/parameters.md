# Parameters

Talos can view and set ROS 2 parameters on any node in the graph. Parameters
belong to individual nodes, so all parameter operations are addressed to a
fully-qualified node name (for example `/talos_agent`).

## How It Works

The agent acts as a parameter service *client*. For a requested node it creates
short-lived clients to that node's standard `rcl_interfaces` parameter services:

- `<node>/list_parameters`
- `<node>/get_parameters`
- `<node>/set_parameters`

These clients are serviced by the bridge node's spinning executor. The agent
waits briefly for the target services to become available, forwards the call,
and converts the result into transport-agnostic protocol types so clients need
no ROS 2 message definitions.

If the bridge node is not yet running, or the target node does not expose
parameter services within the wait window, the agent returns an error rather
than blocking indefinitely.

## Values

Parameter values are carried as `ParamValue`, mirroring the variants of
`rcl_interfaces/msg/ParameterValue`: `Bool`, `Integer`, `Double`, `String`,
byte/bool/integer/double/string arrays, and `NotSet` for parameters that exist
but have no value.

When setting a value from text (CLI argument or TUI edit field), the type is
inferred:

- `true` / `false` &rarr; bool
- an integer literal &rarr; integer
- a number with a decimal point &rarr; double
- `[ ... ]` &rarr; an array (element type inferred from the contents)
- anything else &rarr; string (surrounding quotes are stripped)

## Clients

The CLI exposes `list-params`, `get-param`, and `set-param`. See
[CLI](../usage/cli.md#parameters).

The TUI Params tab lists graph nodes, loads a selected node's parameters, and
can edit a parameter value in place. See [TUI](../usage/tui.md#params).

Setting a parameter reports the node's per-parameter result; a rejected change
includes the reason supplied by the node.
