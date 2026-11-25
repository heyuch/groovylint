
            def someMock = Mock(MyType) {
                getSomeData() >> 'some-data'
            }
            def otherMock = Mock(MyType, [a:1]) {
                getSomeData() >> 'some-data'
            }
        