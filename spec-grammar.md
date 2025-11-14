# DBC file grammar specification

This document describes the structure and grammar used by the DBC parser (`dbcparser` crate).  
The goal is to provide a human-readable and machine-oriented specification that reflects the real-world DBC format (as produced by Vector tools and reverse-engineered by the open-source community).

---

## 1. Overview

DBC files describe messages, signals, and metadata used in CAN communication systems.  
They follow a line-oriented text format with ASCII encoding (occasionally containing UTF-8 comments).  
Each section begins with a keyword followed by one or more arguments separated by spaces.  
Comments start with `//` and extend to the end of the line.

A typical DBC file contains:

```

VERSION "1.0"
NS_:
  NS_DESC_
  CM_
  BA_DEF_
  BA_DEF_DEF_
  BA_
  VAL_
BS_:
BU_: ECU1 ECU2
BO_ 100 ExampleMessage: 8 ECU1
 SG_ ExampleSignal : 0|8@1+ (1,0) [0|255] "unit" ECU2

```

---

## 2. High-level grammar

The grammar below uses an EBNF-like syntax for readability:

```

dbc_file          = header_section, network_section, message_section*, comment_section*, attribute_section*, value_table_section*, new_symbols_section?, version_section?;

header_section    = "VERSION", string_literal, newline;
version_section   = "VERSION", string_literal, newline;
new_symbols_section = "NS_", ":", (symbol_identifier, newline)+;

network_section   = "BS_", ":", optional_value_list, newline;
node_section      = "BU_", ":", node_identifier_list, newline;

message_section   = message_definition, signal_definition*;
message_definition = "BO_", message_id, message_name, ":", dlc, transmitter, newline;
signal_definition  = "SG_", signal_name, ":", bit_start, "|", bit_length, "@", endianness, sign, "(", scale, ",", offset, ")", "[", minimum, "|", maximum, "]", unit, receiver_list?, newline;

comment_section   = "CM_", comment_body, newline;
attribute_section = "BA_DEF_" | "BA_DEF_DEF_" | "BA_" | "VAL_" …;
value_table_section = "VAL_TABLE_", table_name, value_map, newline;

```

---

## 3. Token definitions

```

message_id        = integer;
message_name      = identifier;
signal_name       = identifier;
dlc               = integer;          // data length (0..8)
transmitter       = identifier;
receiver_list     = identifier_list;
endianness        = "0" | "1";       // 0 = Motorola (big endian), 1 = Intel (little endian)
sign              = "+" | "-";       // + = unsigned, - = signed
scale             = float;
offset            = float;
minimum           = float;
maximum           = float;
unit              = string_literal | "";
identifier        = ASCII_ALPHA (ASCII_ALNUM | "_")*;
integer           = ["-"], DIGIT+;
float             = ["-"], DIGIT+, [".", DIGIT+];
string_literal    = '"', { CHAR - '"' }, '"';

```

---

## 4. Section details

### 4.1 VERSION

```

VERSION "1.0"

```

Defines the version of the DBC format or tool that generated it.  
Usually ignored by most parsers but required by the spec.

---

### 4.2 NS_ (New symbols)

```

NS_:
    NS_DESC_
    CM_
    BA_DEF_
    BA_DEF_DEF_
    BA_
    VAL_

```

Declares the types of entries used in this DBC file.  
The list is informational and not mandatory for parsing.

---

### 4.3 BS_ (Bit timing)

```

BS_: 125000

```

Defines the CAN bit timing information (baud rate).  
Often empty (`BS_:` alone).  
This section is optional.

---

### 4.4 BU_ (Node list)

```

BU_: ECU1 ECU2 ECU3

```

Lists all ECU nodes (transmitters and receivers).  
Nodes can be referenced later in `BO_` and `SG_` lines.

---

### 4.5 BO_ (Message definition)

```

BO_<message_id> <message_name>: <dlc> <transmitter>

```

Defines a CAN message (frame):

- `message_id`: 11-bit or 29-bit CAN identifier (integer)
- `message_name`: message symbolic name
- `dlc`: Data Length Code (number of bytes)
- `transmitter`: node that transmits this message

Example:

```

BO_ 100 ExampleMessage: 8 ECU1

```

---

### 4.6 SG_ (Signal definition)

```

SG_<signal_name> : <start_bit>|<signal_size>@<endianness><sign> (<factor>,<offset>) [<min>|<max>] "<unit>" <receiver>

```

- `signal_name`: name of the signal
- `start_bit`: bit index (0..63)
- `signal_size`: bit length
- `endianness`: `0` = Motorola (big endian), `1` = Intel (little endian)
- `sign`: `+` unsigned, `-` signed
- `factor`: scaling factor (multiplier)
- `offset`: additive offset
- `min`, `max`: physical range
- `unit`: optional measurement unit
- `receiver`: node(s) that consume the signal

Example:

```

SG_ ExampleSignal : 0|8@1+ (1,0) [0|255] "V" ECU2

```

Multiplexed signals follow an extended syntax:

```

SG_MuxerSignal M : 0|8@1+ (1,0) [0|255] "" ECU1
SG_ SubSignal m1 : 8|8@1+ (1,0) [0|255] "" ECU2

```

---

### 4.7 CM_ (Comments)

```

CM_"General file comment"
CM_ BO_100 "Message comment"
CM_ SG_ 100 ExampleSignal "Signal comment"

```

- `CM_` on its own applies to the file.
- `CM_ BO_` applies to a message by ID.
- `CM_ SG_` applies to a signal within a message.

---

### 4.8 BA_DEF_, BA_DEF_DEF_, BA_

Attribute definitions and assignments.

```

BA_DEF_ "GenMsgCycleTime" INT 0 10000;
BA_DEF_DEF_"GenMsgCycleTime" 100;
BA_ "GenMsgCycleTime" BO_ 100 500;

```

- `BA_DEF_`: defines an attribute type.
- `BA_DEF_DEF_`: defines its default value.
- `BA_`: assigns an attribute to a specific object (message, signal, node, etc).

---

### 4.9 VAL_ (Value tables)

```

VAL_100 ExampleSignal 0 "Off" 1 "On" 2 "Error";
VAL_TABLE_ SignalState 0 "Off" 1 "On" 2 "Error";

```

Maps numeric values to string labels for a given signal or table.

---

### 4.10 Environment variables (EV_)

```

EV_VarName : 0 [0|1] "unit" 0 0 DUMMY_NODE Vector__XXX;

```

Rarely used, defines environment variables shared between ECUs.

---

## 5. Encoding and syntax rules

- Encoding: ASCII or UTF-8 (without BOM).
- End of line: `\n` or `\r\n`.
- Tabs and extra spaces are ignored between tokens.
- Comments (`// ...`) are ignored.
- Identifiers are case-sensitive.
- Empty sections are allowed (`BS_:` or `BU_:`).
- Semicolons (`;`) terminate some sections (e.g., `BA_`, `VAL_`, `CM_`).

---

## 6. Common irregularities in real-world files

The DBC standard is inconsistently followed.  
Real-world files may contain:

- Extra spaces between tokens.
- Missing semicolons at end of attribute/value lines.
- Windows line endings `\r\n`.
- Empty lines or trailing spaces.
- Invalid or non-ASCII characters in comments.
- Repeated definitions (last one wins).
- Inconsistent case in identifiers.

The parser must be tolerant and robust against these variants.

---

## 7. Validation rules summary

| Validation item              | Condition checked |
|------------------------------|-------------------|
| Unique message IDs           | no duplicate message_id |
| Unique message names         | no duplicate message_name |
| Unique signal names per message | no duplicates within a message |
| Bit range validity           | 0 ≤ start_bit < 64 and start_bit+length ≤ 64 |
| Scaling and offset numeric   | parseable floats |
| Min ≤ Max                    | physical range consistency |
| Endianness flag              | only 0 or 1 |
| Multiplexing rules           | one M (muxer) per message; all m-signals reference valid mux value |
| Receivers and transmitters   | must exist in BU_ list if declared |
| Attribute types              | BA_DEF must exist before BA_ assigns it |

---

## 8. Example minimal DBC file

```

VERSION "1.0"

NS_:
  CM_
  BA_DEF_
  BA_

BS_:
BU_: ECU1 ECU2

BO_100 ExampleMessage: 8 ECU1
 SG_ ExampleSignal : 0|8@1+ (1,0) [0|255] "V" ECU2

CM_SG_ 100 ExampleSignal "Example comment"
BA_DEF_ "GenMsgCycleTime" INT 0 10000;
BA_"GenMsgCycleTime" BO_ 100 100;
VAL_ 100 ExampleSignal 0 "Off" 1 "On";

```

---

## 9. Parser implementation notes

- The lexer should recognize tokens and skip whitespace/comments.
- The parser should map tokens into structured entities (`Dbc`, `Message`, `Signal`, `Attribute`).
- Numeric parsing should handle integers and floats safely.
- Optional tolerance mode can ignore minor violations.
- Span tracking (line/col) should be preserved for error reporting.
- Output structure should be serializable to JSON or YAML (feature-gated).

---

## 10. References

- Vector DBC File Format (unofficial reverse-engineered spec):  
  <https://www.csselectronics.com/pages/can-dbc-file-database-intro>

- BusMaster DBC specification (open-source):  
  <https://raw.githubusercontent.com/rbei-etas/busmaster-documents/master/help.pdf>

- CANdb++ user documentation excerpts:  
  <https://cdn.vector.com/cms/content/products/candb/Docs/CANdb_Manual_EN.pdf>

- ISO 11898-1 (for CAN frame semantics):  
  <https://www.iso.org/standard/63648.html>

---
