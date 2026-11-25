
            [1, 2, 3].each { println it }
            [1, 2, 3].collect {
                it * 2
            }.any {
                it > 5
            }
        