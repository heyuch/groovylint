
            def list = [1, 2, [3, 4, [5, 6]], 7]
            list.collectAll { it * 2 }
            list.collectAll([8, 9]) { it * 2 }
        