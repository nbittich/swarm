import React, { useRef, } from "react";
import "./Favicon.css";

type Props = {
    handleNavigation: (path: string) => void;
};

const FaviconIcon: React.FC<Props> = ({ handleNavigation }) => {
    const audioRef = useRef<HTMLAudioElement | null>(null);
    const imgRef = useRef<HTMLImageElement | null>(null);
    const pressTimer = useRef<NodeJS.Timeout | null>(null);
    const handleEasterOne = () => {
        if (audioRef.current) {
            audioRef.current.currentTime = 0;
            audioRef.current.play();

            if (imgRef.current) {
                imgRef.current.classList.remove("animate-loop");
                void imgRef.current.offsetWidth;
                imgRef.current.classList.add("animate-loop");
            }
        }
    };
    const handlePointerDown = () => {
        pressTimer.current = setTimeout(() => {
            handleEasterOne();
        }, 1000); // 1000ms = 1 second
    };

    const clearPressTimer = () => {
        if (pressTimer.current) {
            clearTimeout(pressTimer.current);
            pressTimer.current = null;
        }
    };
    return (
        <>
            <a onClick={() => handleNavigation("/")}>
                <img
                    onContextMenu={(e) => { e.preventDefault(); handleEasterOne() }}
                    onPointerDown={handlePointerDown}
                    onPointerUp={clearPressTimer}
                    onPointerCancel={clearPressTimer}
                    onPointerLeave={clearPressTimer}
                    className="favicon-icon"
                    ref={imgRef}
                    src="/favicon.svg"
                    width={24}
                    height={24}
                    style={{ verticalAlign: "middle" }}
                />
            </a>
            <audio ref={audioRef} src="/batman.mp3" />
        </>
    );
};

export default FaviconIcon;
