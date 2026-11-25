
            def allPaths = resultsMap.keySet().sort()
            myList.sort()
            myList.sort() { it }
            myList.sort(2) { it }
            myList.findAll{ it < 50 }.sort()
            def w = getMyList().sort().findAll { x < 1 }
            def x = myList.foo.sort().findAll { x < 1 }
            def y = myList.foo().sort().findAll { x < 1 }
        