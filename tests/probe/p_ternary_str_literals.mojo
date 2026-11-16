def classify(x: Int32) -> String:
    var label = "big" if x > 100 else "small"
    return label


def main():
    print(classify(101), classify(1))
