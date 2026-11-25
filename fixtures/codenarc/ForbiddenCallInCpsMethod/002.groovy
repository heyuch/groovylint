
            void run() {
                List l = [4,1,3]
                l.sort { a, b -> a > b } 
                
                l.toSorted { a, b -> a > b } 
                
                "hello".eachLine { line, number -> println(line) }
                "hello".eachLine(2) { line, number -> println(line) }
            }
        