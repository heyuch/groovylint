
            def comparator = { o1, o2 -> o1 <=> o2 }
            def x = myList.unique(true, comparator)
            def y = myList.unique(true) { it }
        