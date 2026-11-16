def main():
    var d = Dict[String, Int]()
    d["a"] = 1
    print(d.get("a").value(), d.pop("a", -1), d.pop("a", -1))
