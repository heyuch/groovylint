
            def comparator = { o1, o2 -> o1 <=> o2 }
            def x = myList.unique(false, comparator)
            def y = myList.unique(false) { it }
        