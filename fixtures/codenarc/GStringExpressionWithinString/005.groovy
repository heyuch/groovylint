
            def valueToBeReplaced = '123'
            def str1 = "123"
            def str2 = "abc def ghi"
            def str3 = "abc ${count}"
            def str4 = "abc $count    }"
            def str5 = "abc {123}"
            def str6 = "abc ${}"
            def str7 = "total: ${count * 25}"
            def str8 = "$valueToBeReplaced $valueNotToBeReplaced"
        