"""Locked native metres, identical on both sides."""

def frame(tag,h):
    if tag=="bight-high":
        return (tag,(-670.,153.,119.),(-353.,6.4,60.),319.,35,1)
    if tag=="bight-tooth":
        # Beside the lowest toe of the actual #1785 erratic, looking along the
        # rising dark northern exposure. Nothing is moved toward the camera.
        x,z,tx,tz=-355.95,40.45,-361.88,20.60
        return (tag,(x,h(x,z)+1.04,z),(tx,h(tx,tz)+.29,tz),0,27,0)
    if tag=="bight-beach":
        # Same cove, its sheltered southern bench, looking to both original
        # boulders #1619 and #1472 against the honest forest edge.
        x,z,tx,tz=-341.62,125.25,-332.55,120.9
        return (tag,(x,h(x,z)+1.04,z),(tx,h(tx,tz)+.58,tz),0,23,0)
    raise ValueError(tag)

