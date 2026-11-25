
            List someList = [1,2,3]
            someList.find { it == b.property }
            someList.findAll { it == b.property }
            someList.findIndexOf { it == b.property }
            someList.every { it == b.property }
            someList.any { it == b.property }
            someList.filter { it == b.property }
            someList.grep { it == b.property }
            someList.dropWhile { it == b.property }
            someList.takeWhile { it == b.property }
            someList.takeWhile { it.property == b }
            someList.takeWhile { it.property.deepProperty == b }
            someList.hest { a = b.property }
            someList.hest { a = b.property }
            someList.find {
                Integer integer = 42
                return it.hest
            }
        