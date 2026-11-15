@fieldwise_init
struct Axis(ImplicitlyCopyable, Equatable, Intable, Writable, Comparable):
    """Axis in a 2D cartesian space."""
    var _tag: Int
    # The y axis
    comptime Vertical = Self(0)
    # The x axis
    comptime Horizontal = Self(1)

    def __eq__(self, other: Self) -> Bool:
        return self._tag == other._tag

    def __ne__(self, other: Self) -> Bool:
        return self._tag != other._tag

    def __int__(self) -> Int:
        return self._tag

    def write_to[W: Writer](self, mut writer: W):
        if self._tag == 0:
            writer.write("Vertical")
        elif self._tag == 1:
            writer.write("Horizontal")

    def __lt__(self, other: Self) -> Bool:
        return self._tag < other._tag

    def __le__(self, other: Self) -> Bool:
        return self._tag <= other._tag

    def __gt__(self, other: Self) -> Bool:
        return self._tag > other._tag

    def __ge__(self, other: Self) -> Bool:
        return self._tag >= other._tag

    def invert(self) -> Self:
        if self == Self.Vertical:
            return Self.Horizontal
        else:
            return Self.Vertical

    def is_vertical(self) -> Bool:
        return self == Self.Vertical

@fieldwise_init
struct Corner(ImplicitlyCopyable, Equatable, Intable, Writable, Defaultable):
    var _tag: Int
    comptime TopLeft = Self(0)
    comptime TopRight = Self(1)
    comptime BottomLeft = Self(4)
    comptime BottomRight = Self(5)

    def __eq__(self, other: Self) -> Bool:
        return self._tag == other._tag

    def __ne__(self, other: Self) -> Bool:
        return self._tag != other._tag

    def __int__(self) -> Int:
        return self._tag

    def write_to[W: Writer](self, mut writer: W):
        if self._tag == 0:
            writer.write("TopLeft")
        elif self._tag == 1:
            writer.write("TopRight")
        elif self._tag == 4:
            writer.write("BottomLeft")
        elif self._tag == 5:
            writer.write("BottomRight")

    def __init__(out self):
        self = Self.TopLeft

def corner_code(c: Corner) -> Int32:
    if c == Corner.TopLeft or c == Corner.TopRight:
        return 1
    elif c == Corner.BottomLeft:
        return 2
    else:
        return 3
