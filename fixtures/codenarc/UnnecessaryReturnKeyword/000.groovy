
            def x = { y++; it }
            def x = { it }
            def method1(it) {
                y++
                it
            }
            def method2(it) {
                it
            }
            def method3() {
                return { 5 }
            }
        