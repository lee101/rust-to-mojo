def main():
    var l: List[Int] = [1, 2, 3]
    for ref x in l:
        x += 1
    print(l[0], l[1], l[2])
