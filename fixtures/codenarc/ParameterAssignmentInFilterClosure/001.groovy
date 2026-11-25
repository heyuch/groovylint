
            List someList = [1, 2, 3]
            List someOtherList = [object1, object2, object3]
            someList.find { it = 4 }
            someList.findAll { it = 42 }
            someList.findIndexOf { it = 42 }
            someList.every { it = 42 }
            someList.any { it = 42 }
            someList.filter { it = 42 }
            someList.grep { it = 42 }
            someList.dropWhile { it = 42 }
            someList.takeWhile { it = 42 }
            someOtherList.takeWhile { it.someProperty = 42 }
            someOtherList.takeWhile { it.someProperty.aDeeperProperty = 42 }
            someList.find { Integer integer ->
                integer = 42
            }
        