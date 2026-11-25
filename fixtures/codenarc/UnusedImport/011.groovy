
            import test.TestData1
            import test.TestData2
            import test.TestData3
            import test.TestData4
            import test.TestData5
            import test.TestData6
            import test.TestData7
            import test.TestData8
            import test.TestData9
            import test.TestData10
            import test.TestData11
            import test.TestData12
            import test.TestData13
            import test.TestData14
            import test.TestData15

            def GSTRING1 = " ${TestData1.GOOD_XML}"
            def GSTRING2 = " $TestData2.XML"
            def MAP1 = [(TestData3):123]
            def MAP2 = [abc:TestData4]
            def MAP3 = [abc:TestData5, ddd:123]
            def LIST = [TestData6,TestData7]
            def OPERATORS1 = 0+TestData8.VALUE-TestData9.VALUE
            def OPERATORS2 = 9*TestData10.VALUE/TestData11.VALUE
            def OPERATORS3 = 64&TestData12.VALUE|TestData13.VALUE^TestData14.VALUE
            def OPERATORS4 = !TestData15.VALUE
        